# VOS

Bare-metal UEFI operating system written in Rust.

```
  _    _____  ___
 | |  / / __ \/ __|
 | | / / /_/ /\__ \
 | |/ / ____/ ___) |
 |___/_/    /____/

 VOS v0.1.0 - UEFI Shell
 Type 'help' for available commands.

vos>
```

## Features

- UEFI boot on **x86_64** and **aarch64**
- Interactive shell with line editing (backspace, typed echo)
- Built-in commands: `help`, `echo`, `info`, `clear`, `reboot`
- Color output (prompt, errors, banner)
- Runs in QEMU or on real UEFI hardware

## Shell Demo

```
vos> help
Available commands:
  help    - show this message
  echo    - echo text back
  clear   - clear screen
  info    - show system info
  reboot  - reboot the system

vos> info
VOS v0.1.0
Firmware: EDK II (rev 65536)
UEFI: 2.70
Console: 80x50 (mode 1)

vos> echo Hello, World!
Hello, World!

vos> reboot
Rebooting...
```

## Prerequisites

- **Rust nightly** with `rust-src` component
- **QEMU** with EDK2 UEFI firmware
- **macOS** (for disk image creation via `hdiutil`/`diskutil`)

### Install on macOS

```bash
# Rust nightly
rustup install nightly
rustup default nightly
rustup component add rust-src llvm-tools

# QEMU
brew install qemu
```

Verify EDK2 firmware is present:

```bash
ls /opt/homebrew/share/qemu/edk2-aarch64-code.fd
ls /opt/homebrew/share/qemu/edk2-x86_64-code.fd
```

## Build & Run

### aarch64 (recommended on Apple Silicon)

```bash
cargo build --target aarch64-unknown-uefi
bash create_uefi_img.sh aarch64
bash run_qemu.sh aarch64
```

### x86_64

```bash
cargo build --target x86_64-unknown-uefi
bash create_uefi_img.sh x86_64
bash run_qemu.sh x86_64
```

> **Note:** On Apple Silicon, x86_64 runs under QEMU TCG emulation (slower than native aarch64).

Once QEMU starts, the VOS shell appears in your terminal. Type commands and press Enter.

Press `Ctrl+C` to kill QEMU when done.

## Project Structure

```
src/
├── main.rs          # UEFI entry point (efi_main)
├── lib.rs           # Library root, module exports
├── shell.rs         # Interactive shell (commands, line editor)
└── arch/
    ├── mod.rs       # Architecture dispatcher
    ├── x86_64/      # x86_64-specific code (GDT, IDT, serial, VGA)
    └── aarch64/     # aarch64-specific code
```

## How It Works

1. UEFI firmware loads `BOOTAA64.EFI` (or `BOOTX64.EFI`) from FAT32 partition
2. `efi_main()` initializes UEFI services (allocator, logger)
3. Shell loop starts: prints prompt, reads keyboard input, dispatches commands
4. All I/O goes through UEFI console protocols, mapped to serial by QEMU (`-serial stdio`)

## License

MIT
