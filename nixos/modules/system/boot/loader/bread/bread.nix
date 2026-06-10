{
  config,
  lib,
  pkgs,
  ...
}:

let
  cfg = config.boot.loader.bread;
  efi = config.boot.loader.efi;

  efiFileForArch = {
    x86_64 = "BOOTX64.EFI";
    aarch64 = "BOOTAA64.EFI";
    riscv64 = "BOOTRISCV64.EFI";
  };

  arch = pkgs.stdenv.hostPlatform.uname.processor;
  efiFile = efiFileForArch.${arch} or (throw "boot.loader.bread: unsupported architecture: ${arch}");

  # Normalise typed extraEntries submodules and raw extraEntriesAttrs into one JSON array.
  allExtraEntries =
    map (
      e:
      {
        inherit (e) type id name;
      }
      // lib.optionalAttrs (e.type == "linux") (
        { inherit (e) kernel initrd; } // lib.optionalAttrs (e.cmdline != null) { inherit (e) cmdline; }
      )
      // lib.optionalAttrs (e.type == "efi") { inherit (e) path; }
    ) cfg.extraEntries
    ++ cfg.extraEntriesAttrs;

  # Pre-populated Deno module cache for bread-install.ts's jsr:@std imports.
  # This is a fixed-output derivation (FOD) that fetches all remote sources
  # once at Nix evaluation / build time so the installer can run offline
  # inside the NixOS disk-image VM (which has no network access).
  # Only the content-addressed remote/ tree is kept; the sqlite caches are
  # intentionally excluded because they are not needed for --cached-only runs
  # and their WAL files are non-deterministic.
  # To update: delete the hash, run `nix build`, and paste the new hash.
  denoCache = pkgs.stdenvNoCC.mkDerivation {
    name = "bread-install-deno-cache";
    nativeBuildInputs = [ pkgs.deno ];
    outputHashAlgo = "sha256";
    outputHashMode = "recursive";
    outputHash = "sha256-HNcBxAv/KzLT/n4eXayj1XXYf5KmSYaHoEurqXttcfs=";

    # No source archive to unpack; the script is passed directly as a store path.
    dontUnpack = true;

    buildPhase = ''
      export DENO_DIR="$PWD/deno-cache"
      deno cache --no-npm ${./bread-install.ts}
    '';

    installPhase = ''
      # Copy only the content-addressed remote/ tree.
      mkdir -p "$out"
      cp -r "$DENO_DIR/remote" "$out/"
    '';
  };

  # JSON config file passed to bread-install.ts at install time.
  # All install parameters live here — no string-substitution into the script.
  installConfigFile = pkgs.writeText "bread-install-config.json" (
    builtins.toJSON {
      efiSysMountPoint = efi.efiSysMountPoint;
      efiBinary = "${cfg.package}/share/bread/bread.efi";
      efiFile = efiFile;
      timeout = cfg.timeout;
      rememberLast = cfg.rememberLast;
      maxGenerations = cfg.maxGenerations;
      font = cfg.font;
      canTouchEfiVariables = efi.canTouchEfiVariables;
      efibootmgrBin = "${pkgs.efibootmgr}/bin/efibootmgr";
      removable = cfg.efiInstallAsRemovable;
      extraEntries = allExtraEntries;
    }
  );

  installScript = pkgs.writeScript "bread-install-boot-loader" ''
    #!${pkgs.runtimeShell}
    export DENO_DIR=${denoCache}
    exec ${pkgs.deno}/bin/deno run \
      --cached-only \
      --allow-read --allow-write --allow-run --allow-env \
      ${./bread-install.ts} \
      ${installConfigFile} "$@"
  '';
in
{
  options.boot.loader.bread = {
    enable = lib.mkEnableOption "the bread UEFI bootloader";

    package = lib.mkOption {
      type = lib.types.package;
      default = pkgs.bread;
      defaultText = lib.literalExpression "pkgs.bread";
      description = ''
        The bread bootloader package to use.
        Provided automatically when using the bread flake overlay.
      '';
    };

    timeout = lib.mkOption {
      type = lib.types.nullOr lib.types.ints.unsigned;
      default = 5;
      example = 10;
      description = ''
        Seconds to wait before auto-booting the default entry.
        Set to {val}`null` to wait indefinitely.
      '';
    };

    rememberLast = lib.mkOption {
      type = lib.types.bool;
      default = false;
      description = ''
        Whether bread should remember and auto-boot the last chosen entry across reboots.
      '';
    };

    maxGenerations = lib.mkOption {
      type = lib.types.nullOr lib.types.ints.positive;
      default = null;
      example = 10;
      description = ''
        Maximum number of NixOS generations to show in the boot menu.
        {val}`null` means no limit — all non-garbage-collected generations are shown.
      '';
    };

    font = lib.mkOption {
      type = lib.types.nullOr lib.types.path;
      default = null;
      example = lib.literalExpression "./terminus32.psf";
      description = ''
        Path to a PSF2 font file to use in the boot menu.
        {val}`null` uses the built-in 8×16 Terminus font embedded in bread.
      '';
    };

    efiInstallAsRemovable = lib.mkOption {
      type = lib.types.bool;
      default = false;
      description = ''
        When {val}`true`, install bread to the removable media path
        ({file}`EFI/BOOT/<arch>.EFI`) instead of {file}`EFI/bread/bread.efi`,
        and skip efibootmgr registration. Useful for firmware that ignores boot
        order, or for removable media.
      '';
    };

    extraEntries = lib.mkOption {
      type = lib.types.listOf (
        lib.types.submodule {
          options = {
            type = lib.mkOption {
              type = lib.types.enum [
                "linux"
                "efi"
              ];
              description = "Entry type.";
            };
            id = lib.mkOption {
              type = lib.types.str;
              description = "Unique identifier for this entry.";
            };
            name = lib.mkOption {
              type = lib.types.str;
              description = "Human-readable label shown in the boot menu.";
            };
            kernel = lib.mkOption {
              type = lib.types.str;
              default = "";
              description = "ESP-relative path to the kernel (required for {val}`linux` entries).";
            };
            initrd = lib.mkOption {
              type = lib.types.listOf lib.types.str;
              default = [ ];
              description = "ESP-relative initrd paths, concatenated in order.";
            };
            cmdline = lib.mkOption {
              type = lib.types.nullOr lib.types.str;
              default = null;
              description = "Kernel command line ({val}`null` omits the field).";
            };
            path = lib.mkOption {
              type = lib.types.str;
              default = "";
              description = "ESP-relative EFI binary path (required for {val}`efi` entries).";
            };
          };
        }
      );
      default = [ ];
      example = lib.literalExpression ''
        [
          { type = "efi"; id = "shell"; name = "UEFI Shell"; path = "/EFI/tools/shell.efi"; }
        ]
      '';
      description = ''
        Extra boot entries as typed submodules, appended after the auto-generated NixOS entries.
        For {val}`folder` entries or other shapes, use {option}`extraEntriesAttrs`.
      '';
    };

    extraEntriesAttrs = lib.mkOption {
      type = lib.types.listOf lib.types.attrs;
      default = [ ];
      example = lib.literalExpression ''
        [
          {
            type    = "folder";
            id      = "tools";
            name    = "Tools";
            entries = [ { type = "efi"; id = "shell"; name = "UEFI Shell"; path = "/EFI/tools/shell.efi"; } ];
          }
        ]
      '';
      description = ''
        Extra boot entries as raw attribute sets (no type-checking).
        Use this for {val}`folder` entries or any shape not covered by {option}`extraEntries`.
        Values must be JSON-serialisable.
      '';
    };
  };

  config = lib.mkIf cfg.enable {
    assertions = [
      {
        assertion = builtins.hasAttr arch efiFileForArch;
        message = "boot.loader.bread: unsupported architecture '${arch}'. Supported: ${lib.concatStringsSep ", " (builtins.attrNames efiFileForArch)}.";
      }
    ];

    boot.loader.grub.enable = lib.mkDefault false;
    boot.loader.supportsInitrdSecrets = true;

    system = {
      boot.loader.id = "bread";
      build.installBootLoader = installScript;
    };
  };
}
