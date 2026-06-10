_:
    @just --list

build:
    cargo build

build-release:
    cargo build --release

[working-directory('vm')]
build-vm:
    rm -rf nixos.qcow2 result nixos-efi-vars.fd
    nixos-rebuild build-vm-with-bootloader --flake ..#bread-vm

[working-directory('vm')]
run-vm: build-vm
    ./result/bin/run-nixos-vm
