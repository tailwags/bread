{ pkgs, ... }:
{
  boot = {
    kernelPackages = pkgs.linuxPackages_latest;

    loader = {
      efi.canTouchEfiVariables = true;

      bread = {
        enable = true;
        timeout = 5;
        rememberLast = false;
        maxGenerations = 5;
        firmwareSetup = true;

        additionalFiles."memtest86/memtest.efi" = pkgs.memtest86plus.efi;
        extraEntries = [
          {
            type = "efi";
            id = "memtest";
            name = "Memtest86+";
            path = "\\EFI\\bread\\files\\memtest86\\memtest.efi";
          }
        ];
      };
    };
  };

  users.users.bread = {
    isNormalUser = true;
    extraGroups = [ "wheel" ];
    initialPassword = "0000";
  };

  virtualisation.vmVariantWithBootLoader = {
    virtualisation.memorySize = 2048;
    virtualisation.cores = 2;
    virtualisation.useEFIBoot = true;

    virtualisation.qemu.options = [
      "-vga none"
      "-device virtio-vga-gl"
      "-display gtk,gl=on"
    ];

    hardware.graphics.enable = true;
  };

  system.stateVersion = "26.11";
}
