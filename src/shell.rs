extern crate alloc;

use alloc::string::String;
use core::fmt::Write;
use uefi::proto::console::text::{Color, Key, ScanCode};
use uefi::runtime::ResetType;
use uefi::{boot, system, Status};

fn print(s: &str) {
    system::with_stdout(|stdout| {
        let _ = stdout.write_str(s);
    });
}

fn println(s: &str) {
    system::with_stdout(|stdout| {
        let _ = stdout.write_str(s);
        let _ = stdout.write_str("\n");
    });
}

fn read_line() -> String {
    let mut buf = String::new();

    loop {
        // Wait for a key event
        let event = system::with_stdin(|stdin| stdin.wait_for_key_event());
        if let Some(event) = event {
            let mut events = [event];
            let _ = boot::wait_for_event(&mut events);
        }

        let key = system::with_stdin(|stdin| stdin.read_key());

        match key {
            Ok(Some(Key::Printable(c))) => {
                let ch: char = match c.try_into() {
                    Ok(ch) => ch,
                    Err(_) => continue,
                };

                if ch == '\r' || ch == '\n' {
                    print("\n");
                    return buf;
                }

                // Backspace (0x08)
                if ch == '\u{8}' {
                    if !buf.is_empty() {
                        buf.pop();
                        // Move cursor back, overwrite with space, move back again
                        print("\u{8} \u{8}");
                    }
                    continue;
                }

                if ch >= ' ' {
                    buf.push(ch);
                    system::with_stdout(|stdout| {
                        let _ = write!(stdout, "{}", ch);
                    });
                }
            }
            Ok(Some(Key::Special(ScanCode::DELETE))) => {
                // Treat Delete like backspace for simplicity
                if !buf.is_empty() {
                    buf.pop();
                    print("\u{8} \u{8}");
                }
            }
            _ => {}
        }
    }
}

fn cmd_help() {
    println("Available commands:");
    println("  help    - show this message");
    println("  echo    - echo text back");
    println("  clear   - clear screen");
    println("  info    - show system info");
    println("  reboot  - reboot the system");
}

fn cmd_info() {
    let fw_vendor = system::firmware_vendor();
    let fw_rev = system::firmware_revision();
    let uefi_rev = system::uefi_revision();

    system::with_stdout(|stdout| {
        let _ = writeln!(stdout, "VOS v0.1.0");
        let _ = writeln!(stdout, "Firmware: {} (rev {})", fw_vendor, fw_rev);
        let _ = writeln!(
            stdout,
            "UEFI: {}.{}",
            uefi_rev.major(),
            uefi_rev.minor()
        );

        if let Ok(Some(mode)) = stdout.current_mode() {
            let _ = writeln!(
                stdout,
                "Console: {}x{} (mode {})",
                mode.columns(),
                mode.rows(),
                mode.index()
            );
        }
    });
}

fn cmd_echo(args: &str) {
    println(args);
}

fn cmd_clear() {
    system::with_stdout(|stdout| {
        let _ = stdout.clear();
    });
}

fn cmd_reboot() -> ! {
    println("Rebooting...");
    uefi::runtime::reset(ResetType::COLD, Status::SUCCESS, None);
}

pub fn run_shell() -> ! {
    system::with_stdout(|stdout| {
        let _ = stdout.clear();
        let _ = stdout.set_color(Color::LightCyan, Color::Black);
        let _ = writeln!(stdout, "  _    _____  ___");
        let _ = writeln!(stdout, " | |  / / __ \\/ __|");
        let _ = writeln!(stdout, " | | / / /_/ /\\__ \\");
        let _ = writeln!(stdout, " | |/ / ____/ ___) |");
        let _ = writeln!(stdout, " |___/_/    /____/");
        let _ = writeln!(stdout);
        let _ = stdout.set_color(Color::White, Color::Black);
        let _ = writeln!(stdout, " VOS v0.1.0 - UEFI Shell");
        let _ = writeln!(stdout, " Type 'help' for available commands.");
        let _ = writeln!(stdout);
    });

    loop {
        system::with_stdout(|stdout| {
            let _ = stdout.set_color(Color::LightGreen, Color::Black);
            let _ = write!(stdout, "vos> ");
            let _ = stdout.set_color(Color::White, Color::Black);
        });

        let line = read_line();
        let line = line.trim();

        if line.is_empty() {
            continue;
        }

        let (cmd, args) = match line.find(' ') {
            Some(i) => (&line[..i], line[i + 1..].trim()),
            None => (line, ""),
        };

        match cmd {
            "help" => cmd_help(),
            "echo" => cmd_echo(args),
            "clear" => cmd_clear(),
            "info" => cmd_info(),
            "reboot" => cmd_reboot(),
            _ => {
                system::with_stdout(|stdout| {
                    let _ = stdout.set_color(Color::Red, Color::Black);
                    let _ = writeln!(stdout, "Unknown command: {}", cmd);
                    let _ = stdout.set_color(Color::White, Color::Black);
                });
            }
        }
    }
}
