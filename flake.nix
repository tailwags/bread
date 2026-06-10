{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    rust-overlay.url = "github:oxalica/rust-overlay";
    rust-overlay.inputs.nixpkgs.follows = "nixpkgs";
    crane.url = "github:ipetkov/crane";
  };

  outputs =
    {
      nixpkgs,
      rust-overlay,
      crane,
      self,
      ...
    }:
    let
      inherit (nixpkgs) lib;

      # The overlay adds pkgs.bread (host-native EFI binary) and applies the
      # rust-overlay so that the Rust toolchain is available for the build.
      # Users can add this to their nixpkgs overlays and then reference
      # pkgs.bread directly in boot.loader.bread.package.
      overlay = lib.composeManyExtensions [
        (import rust-overlay)
        (final: _: {
          bread = final.callPackage ./packages/bread.nix { inherit crane; };
        })
      ];

      eachSystem = lib.genAttrs lib.systems.flakeExposed;
    in
    {
      overlays.default = overlay;

      nixosModules.default = import ./nixos/modules/system/boot/loader/bread/bread.nix;

      packages = eachSystem (
        system:
        let
          pkgs = import nixpkgs {
            inherit system;
            overlays = [ overlay ];
          };
        in
        {
          bread-x86_64 = pkgs.callPackage ./packages/bread.nix {
            inherit crane;
            cargoTarget = "x86_64-unknown-uefi";
          };
          bread-aarch64 = pkgs.callPackage ./packages/bread.nix {
            inherit crane;
            cargoTarget = "aarch64-unknown-uefi";
          };
          default = pkgs.bread;
        }
      );

      devShells = eachSystem (
        system:
        let
          pkgs = import nixpkgs {
            inherit system;
            overlays = [ (import rust-overlay) ];
          };
        in
        {
          default = pkgs.mkShell {
            packages = with pkgs; [
              (rust-bin.fromRustupToolchainFile ./rust-toolchain.toml)
              cargo-nextest
              cargo-expand
              cargo-bloat
              cargo-edit
              just
            ];
          };
        }
      );

      nixosConfigurations.bread-vm = nixpkgs.lib.nixosSystem {
        system = "x86_64-linux";
        modules = [
          ./vm/configuration.nix
          { nixpkgs.overlays = [ overlay ]; }
          self.nixosModules.default
        ];
      };
    };
}
