_:
    @just --list

build:
    cargo build --release

fmt:
    cargo fmt

fmt-check:
    cargo fmt --check

[working-directory('vm')]
build-vm:
    rm -rf nixos.qcow2 result nixos-efi-vars.fd
    nixos-rebuild build-vm-with-bootloader --flake ..#bread-vm

[working-directory('vm')]
run-vm: build-vm
    ./result/bin/run-nixos-vm
