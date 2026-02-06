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
        -serial stdio \
        -display none
elif [ "$ARCH" == "x86_64" ]; then
    echo "Running x86_64 UEFI..."
    qemu-system-x86_64 \
        -machine q35 \
        -m 512M \
        -bios /opt/homebrew/share/qemu/edk2-x86_64-code.fd \
        -drive format=raw,file=vos_uefi_x86_64.img \
        -serial stdio \
        -display none
else
    echo "Unknown arch"
fi
