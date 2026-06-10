{
  pkgs,
  lib,
  crane,
  cargoTarget ?
    if pkgs.stdenv.hostPlatform.isx86_64 then
      "x86_64-unknown-uefi"
    else if pkgs.stdenv.hostPlatform.isAarch64 then
      "aarch64-unknown-uefi"
    else
      throw "bread: unsupported host platform ${pkgs.stdenv.hostPlatform.system}",
  ...
}:

let
  rustToolchain = pkgs.rust-bin.fromRustupToolchainFile ../rust-toolchain.toml;
  craneLib = (crane.mkLib pkgs).overrideToolchain (_: rustToolchain);
in
craneLib.buildPackage {
  pname = "bread";
  version = "0.1.0";

  src = lib.cleanSourceWith {
    src = ../.;
    filter = path: type: craneLib.filterCargoSources path type || lib.hasSuffix ".psf" path;
  };

  cargoVendorDir = craneLib.vendorMultipleCargoDeps {
    cargoLockList = [
      ../Cargo.lock
      "${rustToolchain}/lib/rustlib/src/rust/library/Cargo.lock"
    ];
  };

  strictDeps = true;
  doCheck = false;
  cargoExtraArgs = "--locked --target ${cargoTarget}";

  installPhase = ''
    runHook preInstall
    install -Dm644 target/${cargoTarget}/release/bread.efi $out/share/bread/bread.efi
    runHook postInstall
  '';
}
