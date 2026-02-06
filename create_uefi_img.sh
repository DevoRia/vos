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
DMG_TEMP="${IMG_NAME%.img}.dmg"

echo "Creating disk image: $IMG_NAME"

# Remove old images
rm -f "$IMG_NAME" "$DMG_TEMP"

# Create a FAT32-formatted DMG, then convert to raw
hdiutil create -size 64m -fs "MS-DOS FAT32" -volname VOS_EFI -layout NONE "$DMG_TEMP"

# Mount it
DEV=$(hdiutil attach "$DMG_TEMP" | grep "/Volumes/VOS_EFI" | awk '{print $1}')
echo "Attached to $DEV, mounted at /Volumes/VOS_EFI"

# Create directory structure and copy EFI binary
echo "Copying files..."
mkdir -p /Volumes/VOS_EFI/EFI/BOOT
cp "$EFI_FILE" "/Volumes/VOS_EFI/EFI/BOOT/$DEST_NAME"

# Sync and Detach
echo "Detaching..."
sync
hdiutil detach "$DEV"

# Convert DMG to raw image
hdiutil convert "$DMG_TEMP" -format UDTO -o "${IMG_NAME%.img}"
mv "${IMG_NAME%.img}.cdr" "$IMG_NAME"
rm -f "$DMG_TEMP"

echo "Done! Image created at $IMG_NAME"
