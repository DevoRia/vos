#!/bin/bash
set -e

ARCH=$1
if [ -z "$ARCH" ]; then
    echo "Usage: $0 <x86_64|aarch64>"
    exit 1
fi

EFI_FILE=""
DEST_NAME=""

if [ "$ARCH" == "aarch64" ]; then
    EFI_FILE="target/aarch64-unknown-uefi/debug/vos.efi"
    DEST_NAME="BOOTAA64.EFI"
elif [ "$ARCH" == "x86_64" ]; then
    EFI_FILE="target/x86_64-unknown-uefi/debug/vos.efi"
    DEST_NAME="BOOTX64.EFI"
else
    echo "Unknown arch: $ARCH"
    exit 1
fi

if [ ! -f "$EFI_FILE" ]; then
    echo "EFI file not found: $EFI_FILE"
    echo "Run cargo build first!"
    exit 1
fi

IMG_NAME="vos_uefi_${ARCH}.img"

echo "Creating disk image: $IMG_NAME"
dd if=/dev/zero of=$IMG_NAME bs=1m count=64

# Attach disk image
DEV=$(hdiutil attach -nomount $IMG_NAME | head -n 1 | awk '{print $1}')
echo "Attached to $DEV"

# Format as FAT32
echo "Formatting..."
diskutil eraseVolume "MS-DOS FAT32" VOS_EFI $DEV

# Create directory structure
echo "Copying files..."
mkdir -p /Volumes/VOS_EFI/EFI/BOOT
cp "$EFI_FILE" "/Volumes/VOS_EFI/EFI/BOOT/$DEST_NAME"

# Sync and Detach
echo "Detaching..."
sync
hdiutil detach $DEV

echo "Done! Image created at $IMG_NAME"
