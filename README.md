# Bread

An experimental UEFI bootloader for the Linux kernel written in Rust.

Unlike other UEFI-only bootloaders, Bread does not use the kernel UEFI-stub.
Instead, it loads the kernel, the initrd and the boot information itself and
hands off to the kernel using the native Linux boot protocol, the same one that
has been used in traditional BIOS booting for years. Despite that, it stays
small: a single `no_std` binary of under 80 KiB, with a handful of
dependencies.

Bread can:

- Boot Linux directly, without relying on the kernel EFI-stub
- Concatenate multiple initrds into one
- Verify kernels, initrds and other files with a BLAKE3 hash
- Chainload other EFI applications, such as the Windows boot manager or a UEFI
  shell
- Organize entries in nested folders
- Show a graphical menu with a PSF2 font, falling back to the text console when
  no graphics are available

Currently, the following architectures are supported:

- x86-64
- aarch64

> [!WARNING]
> Bread is experimental. On aarch64, device tree patching is not implemented
> yet, so the initrd and the command line are not passed to the kernel.
>
> Bread is provided as is, without warranty of any kind. We hold no
> responsibility for unbootable systems, lost data or anything else that may
> happen from using it. Keep a working bootloader around as a fallback.

## Building

Bread needs a pinned nightly Rust toolchain. The easiest way to get everything
is with Nix: run `nix develop` (or let `direnv` load it) to enter a shell with
the right toolchain and [just](https://just.systems).

Without Nix, install Rust via [rustup](https://rustup.rs/), which will pick up
the toolchain from `rust-toolchain.toml`. Distro-provided Rust versions will not
work.

To build Bread, run `just build` (or `cargo build --release`). This builds both
targets, and the resulting `bread.efi` files can be found in
`target/<arch>-unknown-uefi/release/`. To build only one, pass
`--target x86_64-unknown-uefi` or `--target aarch64-unknown-uefi`.

> [!NOTE]
> Building without the release profile is not supported. Debug builds will fail
> to link.

With Nix, you can also build the package directly:

```
nix build .#bread-x86_64 # or .#bread-aarch64
```

To try it in a VM, run `just run-vm`. This builds a NixOS VM with Bread
installed as its bootloader (see `vm/configuration.nix`) and boots it in QEMU.

## Using it on NixOS

The flake provides a NixOS module and an overlay with the package. Add Bread to
your flake inputs:

```nix
{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";

    bread = {
      url = "github:tailwags/bread";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };
}
```

Then add the overlay, which provides `pkgs.bread`, and the module to your
system:

```nix
nixpkgs.lib.nixosSystem {
  modules = [
    { nixpkgs.overlays = [ bread.overlays.default ]; }
    bread.nixosModules.default
    ./configuration.nix
  ];
}
```

And enable it in your configuration:

```nix
{
  boot.loader.bread.enable = true;
}
```

Make sure to disable your current bootloader, for example with
`boot.loader.systemd-boot.enable = false`. GRUB is disabled for you.

Bread also needs a way to be found by the firmware. Either enable
`boot.loader.efi.canTouchEfiVariables`, so the installer registers it as a boot
entry with efibootmgr, or set `boot.loader.bread.efiInstallAsRemovable`, so it
gets installed to the fallback path (`EFI/BOOT/BOOT<ARCH>.EFI`) instead.

On every `nixos-rebuild`, the installer copies `bread.efi` to the ESP, copies
the kernels and initrds of your generations to `\EFI\bread\kernels`, removes
the ones no longer used and writes `\EFI\bread\bread.conf`. Specialisations
show up in a folder under their generation.

### Options

| Option                                 | Default | Description                                                                                       |
| -------------------------------------- | ------- | ------------------------------------------------------------------------------------------------- |
| `boot.loader.bread.enable`             | `false` | Whether to use Bread as the bootloader.                                                           |
| `boot.loader.bread.package`            | `pkgs.bread` | The Bread package to use. Provided by the overlay.                                           |
| `boot.loader.bread.timeout`            | `5`     | Seconds before the default entry boots. `null` waits forever.                                     |
| `boot.loader.bread.maxGenerations`     | `null`  | Maximum number of generations in the menu. `null` shows all of them.                              |
| `boot.loader.bread.font`               | `null`  | Path to a PSF2 font for the menu. `null` uses the built-in font.                                  |
| `boot.loader.bread.efiInstallAsRemovable` | `false` | Install to `EFI/BOOT/BOOT<ARCH>.EFI` and skip registering a boot entry with efibootmgr.       |
| `boot.loader.bread.extraEntries`       | `[ ]`   | Extra `linux` or `efi` entries, added after the NixOS ones.                                       |
| `boot.loader.bread.extraEntriesAttrs`  | `[ ]`   | Extra entries as raw attribute sets, for folders or anything `extraEntries` does not cover.       |
| `boot.loader.bread.rememberLast`       | `false` | Accepted, but not implemented yet.                                                                |

### Chainloading another OS

To dual boot, add an `efi` entry pointing to the other OS's bootloader. For
example, for Windows:

```nix
{
  boot.loader.bread.extraEntries = [
    {
      type = "efi";
      id = "windows";
      name = "Windows";
      path = "\\EFI\\Microsoft\\Boot\\bootmgfw.efi";
    }
  ];
}
```

Paths are relative to the ESP Bread was loaded from and use backslashes.

## Configuring

On NixOS the config is generated for you. Elsewhere, write
`\EFI\bread\bread.conf` on the ESP yourself. See [CONFIG.md](CONFIG.md) for the
format.

## License

Bread is licensed under the [EUPL-1.2 License](LICENSE-EUPL). All earlier
versions of this project, previously published under the Apache-2.0 License, are
hereby also made available under the EUPL-1.2.

The Nix packaging and the NixOS module (`flake.nix`, `flake.lock`, `.envrc`,
`packages/`, `nixos/` and `vm/`) are licensed under the
[MIT License](LICENSE-MIT).

The built-in font, `src/ui/cozette.psf`, is
a custom build of [Cozette](https://github.com/the-moonwitch/Cozette), copyright
2020 Samhain and contributors, and is licensed under the MIT License. It is
embedded in `bread.efi`.

The licensing of every file is recorded in [REUSE.toml](REUSE.toml), following
the [REUSE](https://reuse.software) specification; the license texts are in
[LICENSES/](LICENSES).
