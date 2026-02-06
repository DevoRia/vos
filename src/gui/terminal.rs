extern crate alloc;

use alloc::vec;
use alloc::vec::Vec;

use super::font::{CHAR_HEIGHT, CHAR_WIDTH};
use super::gop::{Color, Framebuffer};

#[derive(Clone, Copy)]
pub struct Cell {
    pub ch: u8,
    pub fg: Color,
    pub bg: Color,
}

impl Default for Cell {
    fn default() -> Self {
        Self {
            ch: b' ',
            fg: Color::LIGHT_GRAY,
            bg: Color::TERMINAL_BG,
        }
    }
}

pub struct Terminal {
    pub cols: usize,
    pub rows: usize,
    pub cells: Vec<Cell>,
    dirty: Vec<bool>,
    pub cursor_col: usize,
    pub cursor_row: usize,
    prev_cursor_col: usize,
    prev_cursor_row: usize,
    pub current_fg: Color,
    pub current_bg: Color,
    pub origin_x: usize,
    pub origin_y: usize,
}

impl Terminal {
    pub fn new(cols: usize, rows: usize, origin_x: usize, origin_y: usize) -> Self {
        let total = cols * rows;
        Self {
            cols,
            rows,
            cells: vec![Cell::default(); total],
            dirty: vec![true; total],
            cursor_col: 0,
            cursor_row: 0,
            prev_cursor_col: 0,
            prev_cursor_row: 0,
            current_fg: Color::LIGHT_GRAY,
            current_bg: Color::TERMINAL_BG,
            origin_x,
            origin_y,
        }
    }

    pub fn set_color(&mut self, fg: Color, bg: Color) {
        self.current_fg = fg;
        self.current_bg = bg;
    }

    fn mark_cursor_dirty(&mut self) {
        if self.prev_cursor_row < self.rows && self.prev_cursor_col < self.cols {
            self.dirty[self.prev_cursor_row * self.cols + self.prev_cursor_col] = true;
        }
        if self.cursor_row < self.rows && self.cursor_col < self.cols {
            self.dirty[self.cursor_row * self.cols + self.cursor_col] = true;
        }
    }

    pub fn write_byte(&mut self, byte: u8) {
        self.prev_cursor_col = self.cursor_col;
        self.prev_cursor_row = self.cursor_row;

        match byte {
            b'\n' | b'\r' => {
                self.cursor_col = 0;
                self.cursor_row += 1;
                if self.cursor_row >= self.rows {
                    self.scroll_up();
                }
            }
            0x08 => {
                if self.cursor_col > 0 {
                    self.cursor_col -= 1;
                    let idx = self.cursor_row * self.cols + self.cursor_col;
                    self.cells[idx] = Cell {
                        ch: b' ',
                        fg: self.current_fg,
                        bg: self.current_bg,
                    };
                    self.dirty[idx] = true;
                }
            }
            byte => {
                if self.cursor_col >= self.cols {
                    self.cursor_col = 0;
                    self.cursor_row += 1;
                    if self.cursor_row >= self.rows {
                        self.scroll_up();
                    }
                }
                let idx = self.cursor_row * self.cols + self.cursor_col;
                self.cells[idx] = Cell {
                    ch: byte,
                    fg: self.current_fg,
                    bg: self.current_bg,
                };
                self.dirty[idx] = true;
                self.cursor_col += 1;
            }
        }

        self.mark_cursor_dirty();
    }

    pub fn write_str(&mut self, s: &str) {
        for byte in s.bytes() {
            self.write_byte(byte);
        }
    }

    fn scroll_up(&mut self) {
        for row in 1..self.rows {
            for col in 0..self.cols {
                self.cells[(row - 1) * self.cols + col] = self.cells[row * self.cols + col];
            }
        }
        let last_row = self.rows - 1;
        for col in 0..self.cols {
            self.cells[last_row * self.cols + col] = Cell::default();
        }
        self.cursor_row = self.rows - 1;
        // Everything shifted — mark all dirty
        for d in self.dirty.iter_mut() {
            *d = true;
        }
    }

    pub fn clear(&mut self) {
        for cell in self.cells.iter_mut() {
            *cell = Cell::default();
        }
        for d in self.dirty.iter_mut() {
            *d = true;
        }
        self.cursor_col = 0;
        self.cursor_row = 0;
    }

    pub fn mark_all_dirty(&mut self) {
        for d in self.dirty.iter_mut() {
            *d = true;
        }
    }

    pub fn render(&mut self, fb: &mut Framebuffer) {
        for row in 0..self.rows {
            for col in 0..self.cols {
                let idx = row * self.cols + col;
                if !self.dirty[idx] {
                    continue;
                }
                self.dirty[idx] = false;
                let cell = &self.cells[idx];
                let px = self.origin_x + col * CHAR_WIDTH;
                let py = self.origin_y + row * CHAR_HEIGHT;
                super::font::draw_char(fb, cell.ch, px, py, cell.fg, cell.bg);
            }
        }
        // Underscore cursor
        if self.cursor_row < self.rows && self.cursor_col < self.cols {
            let px = self.origin_x + self.cursor_col * CHAR_WIDTH;
            let py = self.origin_y + self.cursor_row * CHAR_HEIGHT;
            fb.fill_rect(px, py + CHAR_HEIGHT - 2, CHAR_WIDTH, 2, Color::LIGHT_GRAY);
        }
    }
}

impl core::fmt::Write for Terminal {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        Terminal::write_str(self, s);
        Ok(())
    }
}
