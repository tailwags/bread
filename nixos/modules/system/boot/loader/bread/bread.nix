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

  # Normalise typed extraEntries submodules and raw extraEntriesAttrs into one list.
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

  # Substituted into bread-install.py, like limine.nix does for limine-install.py.
  # rememberLast is not passed: bread does not implement it yet.
  installConfigFile = pkgs.writeText "bread-install.json" (
    builtins.toJSON {
      nixEnv = "${config.nix.package}/bin/nix-env";
      efibootmgr = "${pkgs.efibootmgr}/bin/efibootmgr";
      b3sum = "${pkgs.b3sum}/bin/b3sum";
      inherit (efi) efiSysMountPoint canTouchEfiVariables;
      efiBinary = "${cfg.package}/share/bread/bread.efi";
      inherit efiFile;
      inherit (cfg) timeout font firmwareSetup additionalFiles;
      maxGenerations = if cfg.maxGenerations == null then 0 else cfg.maxGenerations;
      efiRemovable = cfg.efiInstallAsRemovable;
      extraEntries = allExtraEntries;
    }
  );

  installScript = pkgs.replaceVarsWith {
    src = ./bread-install.py;
    isExecutable = true;
    replacements = {
      inherit (pkgs) python3;
      configPath = installConfigFile;
    };
  };
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
        {val}`null` uses the built-in 6×13 Cozette font embedded in bread,
        which only covers Latin-1. A font with a unicode table can cover
        any characters it has glyphs for.
      '';
    };

    firmwareSetup = lib.mkOption {
      type = lib.types.nullOr lib.types.bool;
      default = null;
      example = true;
      description = ''
        Whether the boot menu and its error screens offer rebooting into the
        firmware's setup UI with the F2 key. Only offered when the firmware
        supports it. {val}`null` leaves the choice to bread, which defaults to
        no.

        The screen for a broken {file}`bread.conf` cannot read this option, so
        it offers F2 whenever the bread package was built with its
        `configErrorFirmwareSetup` argument (the default). To never offer it,
        for example on a locked-down machine, use
        `pkgs.bread.override { configErrorFirmwareSetup = false; }` as
        {option}`boot.loader.bread.package`.
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

    additionalFiles = lib.mkOption {
      type = lib.types.attrsOf lib.types.path;
      default = { };
      example = lib.literalExpression ''
        { "memtest86/memtest.efi" = pkgs.memtest86plus.efi; }
      '';
      description = ''
        Files to copy to the ESP, under {file}`EFI/bread/files`. Each name is a
        path relative to that directory, using forward slashes. Files there that
        are no longer listed are removed.

        Refer to them from {option}`extraEntries` with their ESP path, for
        example {file}`\EFI\bread\files\memtest86\memtest.efi`.
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
        Values must be strings, numbers, booleans or lists of them (written one line per element);
        {val}`entries` holds nested entries. Values cannot contain newlines or leading/trailing whitespace.
      '';
    };
  };

  config = lib.mkIf cfg.enable {
    assertions = [
      {
        assertion = builtins.hasAttr arch efiFileForArch;
        message = "boot.loader.bread: unsupported architecture '${arch}'. Supported: ${lib.concatStringsSep ", " (builtins.attrNames efiFileForArch)}.";
      }
    ]
    ++ map (name: {
      # Empty, "." and ".." components would place the file outside EFI/bread/files
      # or collide with the directory itself.
      assertion =
        !lib.hasInfix "\\" name
        && lib.all (c: !lib.elem c [ "" "." ".." ]) (lib.splitString "/" name);
      message = "boot.loader.bread.additionalFiles: '${name}' must be a relative path with forward slashes and no empty, '.' or '..' components.";
    }) (builtins.attrNames cfg.additionalFiles);

    boot.loader.grub.enable = lib.mkDefault false;
    boot.loader.supportsInitrdSecrets = true;

    system = {
      boot.loader.id = "bread";
      build.installBootLoader = installScript;
    };
  };
}
