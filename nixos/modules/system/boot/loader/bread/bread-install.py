#!@python3@/bin/python3 -BP

# NixOS boot loader installer for bread, modelled on nixpkgs' limine-install.py:
# kernels and initrds are copied to \EFI\bread\kernels under store-path-derived
# names, and files there that no entry references any more are removed.

import ctypes
import filecmp
import json
import ntpath
import os
import re
import shutil
import subprocess
import sys

config = json.load(open("@configPath@"))
esp = config["efiSysMountPoint"]
bread_dir = os.path.join(esp, "EFI", "bread")
kernels_dir = os.path.join(bread_dir, "kernels")
present: set[str] = set()
kept: set[str] = set()


def get_system_path(profile: str, gen: int | None = None) -> str:
    profiles_dir = "/nix/var/nix/profiles"
    if profile != "system":
        profiles_dir += "/system-profiles"
    return os.path.join(profiles_dir, f"{profile}-{gen}-link" if gen else profile)


def get_profiles() -> list[str]:
    profiles_dir = "/nix/var/nix/profiles/system-profiles"
    dirs = os.listdir(profiles_dir) if os.path.isdir(profiles_dir) else []
    return [path for path in dirs if not path.endswith("-link")]


def get_gens(profile: str) -> list[int]:
    output = subprocess.check_output(
        [config["nixEnv"], "--list-generations", "-p", get_system_path(profile),
         "--option", "build-users-group", ""],
        text=True,
    )
    return [int(line.split()[0]) for line in output.splitlines()][-config["maxGenerations"]:]


def commit(tmp: str, path: str) -> None:
    # fsync before renaming, so a crash cannot leave a truncated file on the ESP.
    fd = os.open(tmp, os.O_RDONLY)
    os.fsync(fd)
    os.close(fd)
    os.rename(tmp, path)


def copy_file(src: str, dst: str) -> None:
    if not (os.path.exists(dst) and filecmp.cmp(src, dst, shallow=False)):
        os.makedirs(os.path.dirname(dst), exist_ok=True)
        shutil.copyfile(src, dst + ".tmp")
        commit(dst + ".tmp", dst)


def remove_file(path: str) -> None:
    if os.path.exists(path):
        os.remove(path)


def copy_kernel(path: str) -> str:
    # The store hash makes the name content-addressed: a kernel or initrd
    # shared across generations is copied once.
    path = os.path.realpath(path)
    name = f"{os.path.basename(os.path.dirname(path))}-{os.path.basename(path)}"
    if name not in present:
        copy_file(path, os.path.join(kernels_dir, name))
        present.add(name)
    kept.add(name)
    return f"\\EFI\\bread\\kernels\\{name}"


def config_entry(entry_id: str, boot_json: dict, label: str) -> dict:
    bootspec = boot_json["org.nixos.bootspec.v1"]
    kernel = copy_kernel(bootspec["kernel"])
    initrd = [copy_kernel(bootspec["initrd"])] if bootspec.get("initrd") else []
    initrd += [
        "\\" + ntpath.normpath(p.lstrip("/"))
        for p in boot_json.get("org.nixos.extra-initrd.v1", {}).get("paths", [])
    ]

    if bootspec.get("initrdSecrets"):
        name = os.path.basename(bootspec["toplevel"]) + "-secrets.initrd"
        path = os.path.join(kernels_dir, name)
        remove_file(path + ".tmp")
        old_umask = os.umask(0o137)
        if subprocess.run([bootspec["initrdSecrets"], path + ".tmp"]).returncode != 0:
            print(f'warning: failed to create initrd secrets for "{label}"', file=sys.stderr)
            print("note: if this is an older generation there is nothing to worry about")
        os.umask(old_umask)
        if os.path.exists(path + ".tmp"):
            commit(path + ".tmp", path)
            present.add(name)
            kept.add(name)
            initrd.append(f"\\EFI\\bread\\kernels\\{name}")

    return {
        "type": "linux",
        "id": entry_id,
        "name": label,
        "kernel": kernel,
        "initrd": initrd,
        "cmdline": " ".join(["init=" + bootspec["init"], *bootspec["kernelParams"]]).strip(),
    }


def profile_entry(profile: str, gens: list[int]) -> dict:
    if profile == "system":
        prefix, name = "nixos", "NixOS"
    else:
        prefix, name = f"nixos-profile-{profile}", f"NixOS profile '{profile}'"

    entries = []
    for gen in sorted(gens, reverse=True):
        boot_json_path = os.path.join(get_system_path(profile, gen), "boot.json")
        try:
            with open(boot_json_path) as f:
                boot_json = json.load(f)
        except FileNotFoundError:
            print(f"warning: skipping generation {gen} of {name}: {boot_json_path} does not exist",
                  file=sys.stderr)
            continue
        except ValueError as e:
            sys.exit(f"error: malformed {boot_json_path}: {e}")

        gen_id = f"{prefix}-gen-{gen}"
        entry = config_entry(gen_id, boot_json, f"Generation {gen}" + ("" if entries else " (current)"))
        if specialisations := sorted(boot_json["org.nixos.specialisation.v1"].items()):
            entry = {
                "type": "folder",
                "id": f"{gen_id}-folder",
                "name": f"Generation {gen}",
                "default": gen_id,
                "entries": [entry] + [
                    config_entry(f"{gen_id}-spec-{spec}", spec_json, spec)
                    for spec, spec_json in specialisations
                ],
            }
        entries.append(entry)

    return {
        "type": "folder",
        "id": prefix,
        "name": name,
        "default": entries[0]["id"] if entries else "",
        "entries": entries,
    }


def conf_line(indent: str, key: str, value) -> str:
    # bread trims each line and reads a value to the end of it.
    text = ("yes" if value else "no") if isinstance(value, bool) else str(value)
    if value is None or isinstance(value, (dict, list)) or "\n" in text or text != text.strip():
        raise ValueError(f"bread.conf: cannot write {key} {json.dumps(value)}")
    return f"{indent}{key} {text}"


def to_conf(fields: dict, indent: str = "") -> list[str]:
    """`key value` per field (one per list element, nulls left out), entries
    opened by `<type> <id>` and closed by `end`."""
    lines = []
    for key, value in fields.items():
        if key == "entries":
            for entry in value:
                lines.append(conf_line(indent, entry["type"], entry["id"]))
                lines += to_conf(entry, indent + "  ")
                lines.append(indent + "end")
        elif key not in ("type", "id") and value is not None:
            lines += [conf_line(indent, key, v) for v in (value if isinstance(value, list) else [value])]
    return lines


def install_bootloader() -> None:
    # Only EFI/bread/kernels is garbage-collected: EFI/bread also holds
    # bread.log, which bread itself writes.
    os.makedirs(kernels_dir, exist_ok=True)
    present.update(os.listdir(kernels_dir))

    profiles = [(profile, get_gens(profile)) for profile in ["system", *get_profiles()]]
    if not any(gens for _, gens in profiles):
        # With zero generations every kernel would be garbage-collected.
        sys.exit("error: no system generations found in /nix/var/nix/profiles, "
                 "refusing to remove all boot loader entries")

    entries = [profile_entry(profile, gens) for profile, gens in profiles]
    conf = {
        "timeout": config["timeout"],
        # Left out when null, so bread's own default applies.
        "firmware-setup": config["firmwareSetup"],
        "default": "nixos",
        # The system profile's folder is the default entry, so always write it.
        "entries": [e for e in entries if e["id"] == "nixos" or e["entries"]] + config["extraEntries"],
    }

    font = os.path.join(bread_dir, "bread.psf")
    if config["font"]:
        copy_file(config["font"], font)
        conf["font"] = "\\EFI\\bread\\bread.psf"
    else:
        remove_file(font)

    conf_path = os.path.join(bread_dir, "bread.conf")
    with open(conf_path + ".tmp", "w", encoding="utf-8") as f:
        f.write("\n".join(["# Written by bread-install; changes are overwritten.", *to_conf(conf)]) + "\n")
    commit(conf_path + ".tmp", conf_path)
    print(f"wrote {conf_path}")

    # Written by older installers.
    remove_file(os.path.join(bread_dir, "bread.json"))

    if config["efiRemovable"]:
        efi_path = os.path.join(esp, "EFI", "BOOT", config["efiFile"])
    else:
        efi_path = os.path.join(bread_dir, "bread.efi")
    copy_file(config["efiBinary"], efi_path)
    print(f"installed {efi_path}")

    if config["efiRemovable"]:
        if config["canTouchEfiVariables"]:
            print("note: boot.loader.bread.efiInstallAsRemovable is true, no need to add EFI entry.")
    elif not config["canTouchEfiVariables"]:
        print("warning: boot.loader.efi.canTouchEfiVariables is set to false while "
              "boot.loader.bread.efiInstallAsRemovable is false.\n  This may render the system unbootable.")
    else:
        dev = os.stat(esp).st_dev
        part = os.path.realpath(f"/sys/dev/block/{os.major(dev)}:{os.minor(dev)}")
        with open(os.path.join(part, "partition")) as f:
            part_num = f.read().strip()
        disk = "/dev/" + os.path.basename(os.path.dirname(part))

        def efibootmgr(*args: str) -> str:
            return subprocess.check_output([config["efibootmgr"], *args], stderr=subprocess.STDOUT, text=True)

        output = efibootmgr()
        create = ["-c"]
        # Replace an existing bread entry, keeping its number and BootOrder position.
        if entry := re.search(r"Boot([0-9a-fA-F]{4})\*?\s+bread\b", output):
            boot_order = re.search(r"BootOrder: ((?:[0-9a-fA-F]{4},?)*)", output)
            boot_order = boot_order[1].upper().split(",") if boot_order else []
            if entry[1].upper() in boot_order:
                create = ["-c", "--index", str(boot_order.index(entry[1].upper()))]
            else:
                create = ["-C"]
            create += ["-b", entry[1]]
            efibootmgr("-b", entry[1], "-B")
        efibootmgr(*create, "-d", disk, "-p", part_num, "-l", "\\EFI\\bread\\bread.efi", "-L", "bread")
        print(f"{'updated' if entry else 'created'} EFI boot entry for bread")

    print("removing unused boot files...")
    for name in present - kept:
        os.remove(os.path.join(kernels_dir, name))


def main() -> None:
    try:
        install_bootloader()
    finally:
        # FAT32 has little recovery after a crash, so sync the ESP after each update.
        libc = ctypes.CDLL("libc.so.6", use_errno=True)
        if libc.syncfs(os.open(esp, os.O_RDONLY)) != 0:
            print(f"could not sync {esp}: {os.strerror(ctypes.get_errno())}", file=sys.stderr)


if __name__ == "__main__":
    main()
