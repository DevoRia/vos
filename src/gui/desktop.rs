extern crate alloc;

use alloc::vec;
use alloc::vec::Vec;
use uefi::proto::console::gop::BltPixel;

use super::font::{CHAR_HEIGHT, CHAR_WIDTH};
use super::gop::{Color, Framebuffer, ScreenInfo};
use super::terminal::Terminal;

const TASKBAR_HEIGHT: usize = 32;
const TERMINAL_MARGIN: usize = 16;
const TITLE_BAR_HEIGHT: usize = 24;
const START_BTN_WIDTH: usize = 80;
const MENU_WIDTH: usize = 200;
const MENU_ITEM_HEIGHT: usize = 28;

#[derive(Clone, Copy)]
pub struct Rect {
    pub x: usize,
    pub y: usize,
    pub w: usize,
    pub h: usize,
}

impl Rect {
    pub fn contains(&self, px: i32, py: i32) -> bool {
        px >= self.x as i32
            && px < (self.x + self.w) as i32
            && py >= self.y as i32
            && py < (self.y + self.h) as i32
    }
}

#[derive(Clone, Copy, PartialEq)]
pub enum ClickAction {
    None,
    CloseTerminal,
    ToggleStartMenu,
    MenuTerminal,
    MenuInfo,
    MenuReboot,
}

pub struct Desktop {
    pub fb: Framebuffer,
    pub terminal: Terminal,
    pub screen_w: usize,
    pub screen_h: usize,
    term_x: usize,
    term_y: usize,
    term_w: usize,
    term_h: usize,
    background_cache: Vec<BltPixel>,
    pub needs_full_redraw: bool,
    pub start_menu_open: bool,
    pub mouse_x: i32,
    pub mouse_y: i32,
    // Clickable regions
    pub close_button_rect: Rect,
    pub start_button_rect: Rect,
    menu_rects: [Rect; 3],
    menu_y: usize,
}

impl Desktop {
    pub fn new(screen: ScreenInfo) -> Self {
        let term_x = TERMINAL_MARGIN;
        let term_y = TERMINAL_MARGIN;
        let term_w = screen.width - 2 * TERMINAL_MARGIN;
        let term_h = screen.height - TASKBAR_HEIGHT - 2 * TERMINAL_MARGIN;

        let text_y = term_y + TITLE_BAR_HEIGHT;
        let text_h = term_h - TITLE_BAR_HEIGHT;

        let cols = term_w / CHAR_WIDTH;
        let rows = text_h / CHAR_HEIGHT;

        let fb = Framebuffer::new(screen.width, screen.height);
        let terminal = Terminal::new(cols, rows, term_x, text_y);

        // Pre-compute gradient background
        let mut background_cache = vec![BltPixel::new(0, 0, 0); screen.width * screen.height];
        for y in 0..screen.height {
            let r = (20 + (y * 15) / screen.height) as u8;
            let g = (30 + (y * 25) / screen.height) as u8;
            let b = (60 + (y * 40) / screen.height) as u8;
            let pixel = BltPixel::new(r, g, b);
            for x in 0..screen.width {
                background_cache[y * screen.width + x] = pixel;
            }
        }

        let close_button_rect = Rect {
            x: term_x + term_w - 20,
            y: term_y + 6,
            w: 12,
            h: 12,
        };

        let taskbar_y = screen.height - TASKBAR_HEIGHT;
        let start_button_rect = Rect {
            x: 0,
            y: taskbar_y,
            w: START_BTN_WIDTH,
            h: TASKBAR_HEIGHT,
        };

        let menu_h = 3 * MENU_ITEM_HEIGHT + 8;
        let menu_y = taskbar_y - menu_h;
        let menu_rects = [
            Rect { x: 0, y: menu_y + 4, w: MENU_WIDTH, h: MENU_ITEM_HEIGHT },
            Rect { x: 0, y: menu_y + 4 + MENU_ITEM_HEIGHT, w: MENU_WIDTH, h: MENU_ITEM_HEIGHT },
            Rect { x: 0, y: menu_y + 4 + 2 * MENU_ITEM_HEIGHT, w: MENU_WIDTH, h: MENU_ITEM_HEIGHT },
        ];

        Self {
            fb,
            terminal,
            screen_w: screen.width,
            screen_h: screen.height,
            term_x,
            term_y,
            term_w,
            term_h,
            background_cache,
            needs_full_redraw: true,
            start_menu_open: false,
            mouse_x: (screen.width / 2) as i32,
            mouse_y: (screen.height / 2) as i32,
            close_button_rect,
            start_button_rect,
            menu_rects,
            menu_y,
        }
    }

    pub fn render(&mut self) {
        if self.needs_full_redraw {
            self.render_full();
            self.needs_full_redraw = false;
        } else {
            self.terminal.render(&mut self.fb);
            self.fb.flush();
        }
    }

    pub fn render_full(&mut self) {
        self.draw_background();
        self.draw_terminal_window();
        self.terminal.mark_all_dirty();
        self.terminal.render(&mut self.fb);
        self.draw_taskbar();
        if self.start_menu_open {
            self.draw_start_menu();
        }
        self.fb.flush();
    }

    fn draw_background(&mut self) {
        self.fb.pixels.copy_from_slice(&self.background_cache);
        self.fb.mark_all_dirty();
    }

    fn draw_terminal_window(&mut self) {
        // Title bar
        self.fb.fill_rect(
            self.term_x,
            self.term_y,
            self.term_w,
            TITLE_BAR_HEIGHT,
            Color::TITLE_BAR,
        );

        // Title text
        let title = "VOS Terminal";
        let title_x = self.term_x + 8;
        let title_y = self.term_y + (TITLE_BAR_HEIGHT - CHAR_HEIGHT) / 2;
        super::font::draw_string(
            &mut self.fb,
            title,
            title_x,
            title_y,
            Color::WHITE,
            Color::TITLE_BAR,
        );

        // Close button with hover feedback
        let hover_close = self.close_button_rect.contains(self.mouse_x, self.mouse_y);
        let btn_color = if hover_close {
            Color::BRIGHT_RED
        } else {
            Color::RED
        };
        let r = self.close_button_rect;
        self.fb.fill_rect(r.x, r.y, r.w, r.h, btn_color);

        // Terminal body background
        self.fb.fill_rect(
            self.term_x,
            self.term_y + TITLE_BAR_HEIGHT,
            self.term_w,
            self.term_h - TITLE_BAR_HEIGHT,
            Color::TERMINAL_BG,
        );
    }

    fn draw_taskbar(&mut self) {
        let y = self.screen_h - TASKBAR_HEIGHT;

        // Background
        self.fb
            .fill_rect(0, y, self.screen_w, TASKBAR_HEIGHT, Color::TASKBAR);

        // Top separator
        self.fb
            .fill_rect(0, y, self.screen_w, 1, Color::new(70, 70, 75));

        // Start button with hover
        let hover_start = self.start_button_rect.contains(self.mouse_x, self.mouse_y);
        let btn_bg = if self.start_menu_open || hover_start {
            Color::TASKBAR_HOVER
        } else {
            Color::TASKBAR
        };
        self.fb
            .fill_rect(0, y + 1, START_BTN_WIDTH, TASKBAR_HEIGHT - 1, btn_bg);
        let label_y = y + (TASKBAR_HEIGHT - CHAR_HEIGHT) / 2;
        super::font::draw_string(&mut self.fb, "VOS", 8, label_y, Color::CYAN, btn_bg);

        // Divider
        self.fb.fill_rect(
            START_BTN_WIDTH,
            y + 4,
            1,
            TASKBAR_HEIGHT - 8,
            Color::new(70, 70, 75),
        );

        // Version on right
        let ver = "v0.1.0";
        let ver_x = self.screen_w - ver.len() * CHAR_WIDTH - 8;
        super::font::draw_string(
            &mut self.fb,
            ver,
            ver_x,
            label_y,
            Color::LIGHT_GRAY,
            Color::TASKBAR,
        );
    }

    fn draw_start_menu(&mut self) {
        let menu_h = 3 * MENU_ITEM_HEIGHT + 8;

        // Menu background
        self.fb
            .fill_rect(0, self.menu_y, MENU_WIDTH, menu_h, Color::MENU_BG);

        // Top border
        self.fb
            .fill_rect(0, self.menu_y, MENU_WIDTH, 1, Color::new(70, 70, 75));
        // Right border
        self.fb
            .fill_rect(MENU_WIDTH - 1, self.menu_y, 1, menu_h, Color::new(70, 70, 75));

        let items = ["Terminal", "Info", "Reboot"];
        for (i, item) in items.iter().enumerate() {
            let r = self.menu_rects[i];
            let hover = r.contains(self.mouse_x, self.mouse_y);
            let bg = if hover {
                Color::MENU_HOVER
            } else {
                Color::MENU_BG
            };
            self.fb.fill_rect(r.x, r.y, r.w, r.h, bg);
            let text_y = r.y + (MENU_ITEM_HEIGHT - CHAR_HEIGHT) / 2;
            super::font::draw_string(&mut self.fb, item, 16, text_y, Color::WHITE, bg);
        }
    }

    pub fn handle_click(&mut self, x: i32, y: i32) -> ClickAction {
        // Start menu items (check first if menu is open)
        if self.start_menu_open {
            for (i, r) in self.menu_rects.iter().enumerate() {
                if r.contains(x, y) {
                    self.start_menu_open = false;
                    self.needs_full_redraw = true;
                    return match i {
                        0 => ClickAction::MenuTerminal,
                        1 => ClickAction::MenuInfo,
                        2 => ClickAction::MenuReboot,
                        _ => ClickAction::None,
                    };
                }
            }
            // Click outside menu closes it
            if !self.start_button_rect.contains(x, y) {
                self.start_menu_open = false;
                self.needs_full_redraw = true;
                return ClickAction::None;
            }
        }

        // Start button
        if self.start_button_rect.contains(x, y) {
            self.start_menu_open = !self.start_menu_open;
            self.needs_full_redraw = true;
            return ClickAction::ToggleStartMenu;
        }

        // Close button
        if self.close_button_rect.contains(x, y) {
            return ClickAction::CloseTerminal;
        }

        ClickAction::None
    }
}
