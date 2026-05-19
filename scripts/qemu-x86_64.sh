#!/bin/sh

if [ ! -f OVMF_VARS.fd ]; then
  cp "$OVMF_VARS" OVMF_VARS.fd
  chmod u+w OVMF_VARS.fd
fi

qemu-system-x86_64 \
  -enable-kvm -machine q35 -device intel-iommu -cpu host -m 1G \
  -nic user,model=virtio-net-pci -smp 6 \
  -drive if=pflash,format=raw,readonly=on,file="$OVMF_CODE" \
  -drive if=pflash,format=raw,file=OVMF_VARS.fd \
  -drive format=raw,file=fat:rw:esp \
  -device virtio-vga-gl -display gtk,gl=on
