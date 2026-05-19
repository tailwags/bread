{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    rust-overlay.url = "github:oxalica/rust-overlay";
    rust-overlay.inputs.nixpkgs.follows = "nixpkgs";
  };

  outputs =
    {
      nixpkgs,
      rust-overlay,
      ...
    }:
    let
      inherit (nixpkgs) lib;
      eachSystem = lib.genAttrs lib.systems.flakeExposed;
    in
    {
      devShells = eachSystem (
        system:
        let
          overlays = [ (import rust-overlay) ];
          pkgs = import nixpkgs {
            inherit system overlays;
          };
        in
        {
          default = pkgs.mkShell {
            buildInputs = with pkgs; [
              cargo-nextest
              cargo-expand
              cargo-bloat
              just
              qemu
              (rust-bin.fromRustupToolchainFile ./rust-toolchain.toml)
            ];
            shellHook = ''
              export OVMF_CODE="${pkgs.OVMF.firmware}"
              export OVMF_VARS="${pkgs.OVMF.variables}"
              export AAVMF_CODE="${pkgs.pkgsCross.aarch64-multiplatform.OVMF.firmware}"
              export RISCV_VIRT_CODE="${pkgs.pkgsCross.riscv64.OVMF.firmware}"
            '';
          };
        }
      );
    };
}
