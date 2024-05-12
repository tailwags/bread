#!/bin/sh

qemu-system-x86_64 \
  -enable-kvm -machine q35 -device intel-iommu -cpu host -m 1G \
  -nic user,model=virtio-net-pci -smp 6 \
  -drive if=pflash,format=raw,readonly=on,file=/usr/share/edk2-ovmf/x64/OVMF_CODE.fd \
  -drive if=pflash,format=raw,file=OVMF_VARS.fd \
  -drive format=raw,file=fat:rw:esp \
  -device virtio-vga-gl -display gtk,gl=on
