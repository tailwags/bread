#!/bin/sh

qemu-system-aarch64 \
  -machine virt -smp 6 -cpu max -m 1G \
  -nic user,model=virtio-net-pci \
  -drive if=pflash,format=raw,readonly=on,file=/usr/share/edk2-ovmf/aarch64/QEMU_CODE.fd \
  -drive format=raw,file=fat:rw:esp \
  -device virtio-gpu-gl-pci -display gtk,gl=on
