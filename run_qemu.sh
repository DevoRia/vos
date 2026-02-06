#!/bin/bash
ARCH=$1
if [ -z "$ARCH" ]; then
    ARCH="aarch64" # Default to arm
fi

if [ "$ARCH" == "aarch64" ]; then
    echo "Running AArch64 UEFI..."
    qemu-system-aarch64 \
        -machine virt \
        -cpu cortex-a57 \
        -m 512M \
        -bios /opt/homebrew/share/qemu/edk2-aarch64-code.fd \
        -drive format=raw,file=vos_uefi_aarch64.img \
        -device virtio-gpu-pci \
        -device qemu-xhci \
        -device usb-kbd \
        -device usb-mouse \
        -serial stdio
elif [ "$ARCH" == "x86_64" ]; then
    echo "Running x86_64 UEFI..."
    qemu-system-x86_64 \
        -machine q35 \
        -m 512M \
        -drive if=pflash,format=raw,readonly=on,file=/opt/homebrew/share/qemu/edk2-x86_64-code.fd \
        -drive format=raw,file=vos_uefi_x86_64.img \
        -device qemu-xhci \
        -device usb-kbd \
        -device usb-mouse \
        -serial stdio
else
    echo "Unknown arch: $ARCH (use aarch64 or x86_64)"
fi
