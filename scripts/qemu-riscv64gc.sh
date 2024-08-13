#!/bin/sh

qemu-system-riscv64 \
  -machine virt,acpi=on,pflash0=pflash0 -smp 6 -cpu max -m 1G \
  -nic user,model=virtio-net-pci \
  -device virtio-gpu-gl -display gtk,gl=on -device qemu-xhci -device usb-kbd \
  -blockdev node-name=pflash0,driver=file,read-only=on,filename=RISCV_VIRT_CODE.fd \
  -device virtio-blk-device,drive=hd0 -drive format=raw,file=fat:rw:esp,id=hd0
