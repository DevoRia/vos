extern crate alloc;

use alloc::string::String;
use core::fmt::Write;
use uefi::proto::console::pointer::Pointer;
use uefi::proto::console::text::{Color, Key, ScanCode};
use uefi::runtime::ResetType;
use uefi::boot::{EventType, OpenProtocolAttributes, OpenProtocolParams};
use uefi::{boot, system, Event, Identify, Status};

use crate::gui::desktop::{ClickAction, Desktop};
use crate::gui::gop::{Color as GColor, ScreenInfo};
use crate::gui::mouse::MouseState;

// ── Text mode helpers (for fallback shell) ──

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
                if ch == '\u{8}' {
                    if !buf.is_empty() {
                        buf.pop();
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
                if !buf.is_empty() {
                    buf.pop();
                    print("\u{8} \u{8}");
                }
            }
            _ => {}
        }
    }
}

// ── Shared commands ──

fn cmd_reboot() -> ! {
    uefi::runtime::reset(ResetType::COLD, Status::SUCCESS, None);
}

fn help_text() -> &'static str {
    "Available commands:\n\
     \x20 help    - show this message\n\
     \x20 echo    - echo text back\n\
     \x20 clear   - clear screen\n\
     \x20 info    - show system info\n\
     \x20 mem     - show memory info\n\
     \x20 ls      - list directory (ls [path])\n\
     \x20 cat     - read file (cat <file>)\n\
     \x20 write   - write file (write <file> <text>)\n\
     \x20 mkdir   - create directory\n\
     \x20 rm      - delete file\n\
     \x20 reboot  - reboot the system\n"
}

fn info_text() -> String {
    let fw_vendor = system::firmware_vendor();
    let fw_rev = system::firmware_revision();
    let uefi_rev = system::uefi_revision();
    let mut s = String::new();
    let _ = writeln!(s, "VOS v0.1.0");
    let _ = writeln!(s, "Firmware: {} (rev {})", fw_vendor, fw_rev);
    let _ = writeln!(s, "UEFI: {}.{}", uefi_rev.major(), uefi_rev.minor());
    s
}

fn run_fs_command(cmd: &str, args: &str) -> Result<String, String> {
    match cmd {
        "ls" => crate::fs::cmd_ls(args),
        "cat" => crate::fs::cmd_cat(args),
        "write" => crate::fs::cmd_write(args),
        "mkdir" => crate::fs::cmd_mkdir(args),
        "rm" => crate::fs::cmd_rm(args),
        _ => Err(String::from("Unknown command")),
    }
}

// ── Text mode shell (fallback) ──

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
            "help" => print(help_text()),
            "echo" => println(args),
            "clear" => {
                system::with_stdout(|stdout| {
                    let _ = stdout.clear();
                });
            }
            "info" => print(&info_text()),
            "mem" => {
                let info = crate::memory::get_memory_info();
                print(&crate::memory::format_memory_info(&info));
            }
            "ls" | "cat" | "write" | "mkdir" | "rm" => match run_fs_command(cmd, args) {
                Ok(output) => print(&output),
                Err(e) => {
                    system::with_stdout(|stdout| {
                        let _ = stdout.set_color(Color::Red, Color::Black);
                        let _ = writeln!(stdout, "{}", e);
                        let _ = stdout.set_color(Color::White, Color::Black);
                    });
                }
            },
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

// ── GUI mode shell ──

/// Try to open the UEFI SimplePointer protocol.
/// Uses GetProtocol (not Exclusive) to avoid disconnecting the USB mouse driver.
fn try_open_pointer() -> Option<boot::ScopedProtocol<Pointer>> {
    let handle = boot::get_handle_for_protocol::<Pointer>().ok()?;
    unsafe {
        boot::open_protocol::<Pointer>(
            OpenProtocolParams {
                handle,
                agent: boot::image_handle(),
                controller: None,
            },
            OpenProtocolAttributes::GetProtocol,
        )
        .ok()
    }
}

/// Create a periodic timer event for mouse polling (~60fps)
fn create_timer_event() -> Option<Event> {
    let event = unsafe {
        boot::create_event(
            EventType::TIMER,
            boot::Tpl::APPLICATION,
            None,
            None,
        )
    };
    if let Ok(ref evt) = event {
        // 160,000 * 100ns = 16ms = ~62fps
        let _ = boot::set_timer(evt, boot::TimerTrigger::Periodic(160_000));
    }
    event.ok()
}

/// Render with cursor: erase old cursor, render scene, draw new cursor, flush
fn render_with_cursor(desktop: &mut Desktop, mouse: &mut MouseState) {
    mouse.erase_cursor(&mut desktop.fb);
    desktop.render();
    mouse.draw_cursor(&mut desktop.fb);
    desktop.fb.flush();
}

/// Full render with cursor (for full redraws)
fn render_full_with_cursor(desktop: &mut Desktop, mouse: &mut MouseState) {
    desktop.needs_full_redraw = true;
    render_with_cursor(desktop, mouse);
}

fn handle_mouse_poll(
    desktop: &mut Desktop,
    mouse: &mut MouseState,
    pointer: &mut Option<boot::ScopedProtocol<Pointer>>,
) {
    if let Some(ref mut ptr) = pointer {
        if let Ok(Some(state)) = ptr.read_state() {
            let old_x = mouse.x;
            let old_y = mouse.y;
            mouse.update(&state);

            // Update desktop's mouse position for hover effects
            desktop.mouse_x = mouse.x;
            desktop.mouse_y = mouse.y;

            if mouse.left_clicked {
                let action = desktop.handle_click(mouse.x, mouse.y);
                match action {
                    ClickAction::ToggleStartMenu => {
                        render_full_with_cursor(desktop, mouse);
                    }
                    ClickAction::CloseTerminal => {
                        desktop.terminal.clear();
                        desktop.terminal.write_str("Terminal cleared.\n");
                        render_full_with_cursor(desktop, mouse);
                    }
                    ClickAction::MenuTerminal => {
                        desktop.terminal.set_color(GColor::WHITE, GColor::TERMINAL_BG);
                        desktop.terminal.write_str("Terminal is active.\n");
                        render_full_with_cursor(desktop, mouse);
                    }
                    ClickAction::MenuInfo => {
                        desktop.terminal.write_str(&info_text());
                        render_full_with_cursor(desktop, mouse);
                    }
                    ClickAction::MenuReboot => {
                        desktop.terminal.write_str("Rebooting...\n");
                        render_full_with_cursor(desktop, mouse);
                        cmd_reboot();
                    }
                    ClickAction::None => {
                        if desktop.needs_full_redraw {
                            render_full_with_cursor(desktop, mouse);
                        }
                    }
                }
            } else if mouse.moved(old_x, old_y) {
                // Just redraw cursor at new position
                mouse.erase_cursor(&mut desktop.fb);
                // Check if hover state changed on interactive elements
                let need_redraw = desktop.start_menu_open
                    || desktop.close_button_rect.contains(mouse.x, mouse.y)
                    || desktop.close_button_rect.contains(old_x, old_y)
                    || desktop.start_button_rect.contains(mouse.x, mouse.y)
                    || desktop.start_button_rect.contains(old_x, old_y);

                if need_redraw {
                    desktop.needs_full_redraw = true;
                    desktop.render();
                }
                mouse.draw_cursor(&mut desktop.fb);
                desktop.fb.flush();
            }
        }
    }
}

fn read_line_gui(
    desktop: &mut Desktop,
    mouse: &mut MouseState,
    timer: &Option<Event>,
    pointer: &mut Option<boot::ScopedProtocol<Pointer>>,
) -> String {
    let mut buf = String::new();

    loop {
        // Get keyboard wait event
        let kb_event = system::with_stdin(|stdin| stdin.wait_for_key_event());

        // Build event array
        match (&kb_event, timer) {
            (Some(kb), Some(tmr)) => {
                let mut events = unsafe { [kb.unsafe_clone(), tmr.unsafe_clone()] };
                match boot::wait_for_event(&mut events) {
                    Ok(0) => {
                        // Keyboard event
                        if let Some(result) = handle_key_input(&mut buf, desktop, mouse) {
                            return result;
                        }
                    }
                    Ok(1) => {
                        // Timer event — poll mouse
                        handle_mouse_poll(desktop, mouse, pointer);
                    }
                    _ => {}
                }
            }
            (Some(kb), None) => {
                // No timer/mouse — keyboard only
                let mut events = unsafe { [kb.unsafe_clone()] };
                let _ = boot::wait_for_event(&mut events);
                if let Some(result) = handle_key_input(&mut buf, desktop, mouse) {
                    return result;
                }
            }
            _ => {
                // No keyboard event available (shouldn't happen)
                continue;
            }
        }
    }
}

fn handle_key_input(
    buf: &mut String,
    desktop: &mut Desktop,
    mouse: &mut MouseState,
) -> Option<String> {
    let key = system::with_stdin(|stdin| stdin.read_key());
    match key {
        Ok(Some(Key::Printable(c))) => {
            let ch: char = match c.try_into() {
                Ok(ch) => ch,
                Err(_) => return None,
            };
            if ch == '\r' || ch == '\n' {
                desktop.terminal.write_byte(b'\n');
                render_with_cursor(desktop, mouse);
                return Some(buf.clone());
            }
            if ch == '\u{8}' {
                if !buf.is_empty() {
                    buf.pop();
                    desktop.terminal.write_byte(0x08);
                    render_with_cursor(desktop, mouse);
                }
                return None;
            }
            if ch >= ' ' {
                buf.push(ch);
                desktop.terminal.write_byte(ch as u8);
                render_with_cursor(desktop, mouse);
            }
        }
        Ok(Some(Key::Special(ScanCode::DELETE))) => {
            if !buf.is_empty() {
                buf.pop();
                desktop.terminal.write_byte(0x08);
                render_with_cursor(desktop, mouse);
            }
        }
        _ => {}
    }
    None
}

pub fn run_gui_shell(screen: ScreenInfo) -> ! {
    let mut desktop = Desktop::new(screen);
    let mut mouse = MouseState::new(screen.width, screen.height);

    // Find all SimplePointer handles and log them
    if let Ok(handles) = boot::locate_handle_buffer(boot::SearchType::ByProtocol(
        &Pointer::GUID,
    )) {
        log::info!("Found {} SimplePointer handles", handles.len());
    }

    // Open pointer protocol once and keep it open
    let mut pointer = try_open_pointer();
    if let Some(ref ptr) = pointer {
        let mode = ptr.mode();
        let res_x = mode.resolution[0];
        mouse.set_sensitivity(1);
        log::info!("Mouse detected (resolution: {}x{}, using raw values)", res_x, mode.resolution[1]);
    } else {
        log::info!("No mouse detected, keyboard only");
    }

    // Create timer for mouse polling
    let timer = create_timer_event();

    // Banner
    desktop
        .terminal
        .set_color(GColor::CYAN, GColor::TERMINAL_BG);
    desktop.terminal.write_str("  _    _____  ___\n");
    desktop.terminal.write_str(" | |  / / __ \\/ __|\n");
    desktop.terminal.write_str(" | | / / /_/ /\\__ \\\n");
    desktop.terminal.write_str(" | |/ / ____/ ___) |\n");
    desktop.terminal.write_str(" |___/_/    /____/\n");
    desktop.terminal.write_str("\n");
    desktop
        .terminal
        .set_color(GColor::WHITE, GColor::TERMINAL_BG);
    desktop.terminal.write_str(" VOS v0.1.0 - UEFI GUI Shell\n");
    desktop
        .terminal
        .write_str(" Type 'help' for available commands.\n\n");

    // Initial full render with cursor
    render_full_with_cursor(&mut desktop, &mut mouse);

    loop {
        desktop
            .terminal
            .set_color(GColor::GREEN, GColor::TERMINAL_BG);
        desktop.terminal.write_str("vos> ");
        desktop
            .terminal
            .set_color(GColor::LIGHT_GRAY, GColor::TERMINAL_BG);
        render_with_cursor(&mut desktop, &mut mouse);

        let line = read_line_gui(&mut desktop, &mut mouse, &timer, &mut pointer);
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        let (cmd, args) = match line.find(' ') {
            Some(i) => (&line[..i], line[i + 1..].trim()),
            None => (line, ""),
        };

        match cmd {
            "help" => {
                desktop.terminal.write_str(help_text());
            }
            "echo" => {
                desktop.terminal.write_str(args);
                desktop.terminal.write_byte(b'\n');
            }
            "clear" => {
                desktop.terminal.clear();
                desktop.needs_full_redraw = true;
            }
            "info" => {
                desktop.terminal.write_str(&info_text());
            }
            "mem" => {
                let info = crate::memory::get_memory_info();
                desktop
                    .terminal
                    .write_str(&crate::memory::format_memory_info(&info));
            }
            "ls" | "cat" | "write" | "mkdir" | "rm" => match run_fs_command(cmd, args) {
                Ok(output) => desktop.terminal.write_str(&output),
                Err(e) => {
                    desktop
                        .terminal
                        .set_color(GColor::RED, GColor::TERMINAL_BG);
                    desktop.terminal.write_str(&e);
                    desktop.terminal.write_byte(b'\n');
                    desktop
                        .terminal
                        .set_color(GColor::LIGHT_GRAY, GColor::TERMINAL_BG);
                }
            },
            "reboot" => {
                desktop.terminal.write_str("Rebooting...\n");
                render_with_cursor(&mut desktop, &mut mouse);
                cmd_reboot();
            }
            _ => {
                desktop
                    .terminal
                    .set_color(GColor::RED, GColor::TERMINAL_BG);
                let mut msg = String::from("Unknown command: ");
                msg.push_str(cmd);
                msg.push('\n');
                desktop.terminal.write_str(&msg);
                desktop
                    .terminal
                    .set_color(GColor::LIGHT_GRAY, GColor::TERMINAL_BG);
            }
        }

        render_with_cursor(&mut desktop, &mut mouse);
    }
}
