build:
    cargo build --release
    
qemu-x86_64: build
    mkdir -p esp/EFI/BOOT
    cp target/x86_64-unknown-uefi/release/bread.efi esp/EFI/BOOT/BOOTX64.EFI
    ./scripts/qemu-x86_64.sh

qemu-aarch64: build
    mkdir -p esp/EFI/BOOT
    cp target/aarch64-unknown-uefi/release/bread.efi esp/EFI/BOOT/BOOTAA64.EFI
    ./scripts/qemu-aarch64.sh

qemu-riscv64gc: build
    mkdir -p esp/EFI/BOOT
    cp target/riscv64gc-unknown-uefi/release/bread.efi esp/EFI/BOOT/BOOTRISCV64.EFI
    ./scripts/qemu-riscv64gc.sh
