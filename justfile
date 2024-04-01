build:
    cargo build --release --target x86_64-unknown-uefi

qemu: build
    mkdir -p esp/EFI/BOOT
    cp target/x86_64-unknown-uefi/release/bread.efi esp/EFI/BOOT/BOOTX64.EFI
    ./scripts/qemu.sh
