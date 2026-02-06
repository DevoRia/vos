extern crate alloc;

use alloc::vec;
use alloc::vec::Vec;
use uefi::proto::console::gop::BltPixel;
use uefi::proto::console::pointer::PointerState;

use super::gop::{Color, Framebuffer};

pub const CURSOR_WIDTH: usize = 12;
pub const CURSOR_HEIGHT: usize = 19;

/// Standard arrow cursor bitmap: 0=transparent, 1=black outline, 2=white fill
static CURSOR_BITMAP: [[u8; CURSOR_WIDTH]; CURSOR_HEIGHT] = [
    [1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
    [1, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
    [1, 2, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0],
    [1, 2, 2, 1, 0, 0, 0, 0, 0, 0, 0, 0],
    [1, 2, 2, 2, 1, 0, 0, 0, 0, 0, 0, 0],
    [1, 2, 2, 2, 2, 1, 0, 0, 0, 0, 0, 0],
    [1, 2, 2, 2, 2, 2, 1, 0, 0, 0, 0, 0],
    [1, 2, 2, 2, 2, 2, 2, 1, 0, 0, 0, 0],
    [1, 2, 2, 2, 2, 2, 2, 2, 1, 0, 0, 0],
    [1, 2, 2, 2, 2, 2, 2, 2, 2, 1, 0, 0],
    [1, 2, 2, 2, 2, 2, 1, 1, 1, 1, 1, 0],
    [1, 2, 2, 1, 2, 2, 1, 0, 0, 0, 0, 0],
    [1, 2, 1, 0, 1, 2, 2, 1, 0, 0, 0, 0],
    [1, 1, 0, 0, 1, 2, 2, 1, 0, 0, 0, 0],
    [1, 0, 0, 0, 0, 1, 2, 2, 1, 0, 0, 0],
    [0, 0, 0, 0, 0, 1, 2, 2, 1, 0, 0, 0],
    [0, 0, 0, 0, 0, 0, 1, 2, 2, 1, 0, 0],
    [0, 0, 0, 0, 0, 0, 1, 2, 1, 0, 0, 0],
    [0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0],
];

pub struct MouseState {
    pub x: i32,
    pub y: i32,
    pub left_button: bool,
    pub right_button: bool,
    pub left_clicked: bool,
    pub screen_w: i32,
    pub screen_h: i32,
    save_buffer: Vec<BltPixel>,
    save_x: usize,
    save_y: usize,
    save_w: usize,
    save_h: usize,
    pub visible: bool,
    sensitivity: i32,
}

impl MouseState {
    pub fn new(screen_w: usize, screen_h: usize) -> Self {
        Self {
            x: (screen_w / 2) as i32,
            y: (screen_h / 2) as i32,
            left_button: false,
            right_button: false,
            left_clicked: false,
            screen_w: screen_w as i32,
            screen_h: screen_h as i32,
            save_buffer: vec![BltPixel::new(0, 0, 0); CURSOR_WIDTH * CURSOR_HEIGHT],
            save_x: 0,
            save_y: 0,
            save_w: 0,
            save_h: 0,
            visible: false,
            sensitivity: 1000,
        }
    }

    pub fn update(&mut self, state: &PointerState) {
        let prev_left = self.left_button;

        // Apply relative movement with scaling
        let dx = state.relative_movement[0] / self.sensitivity;
        let dy = state.relative_movement[1] / self.sensitivity;

        self.x = (self.x + dx).clamp(0, self.screen_w - 1);
        self.y = (self.y + dy).clamp(0, self.screen_h - 1);

        self.left_button = state.button[0];
        self.right_button = state.button[1];
        self.left_clicked = self.left_button && !prev_left;
    }

    pub fn set_sensitivity(&mut self, val: i32) {
        if val > 0 {
            self.sensitivity = val;
        }
    }

    pub fn erase_cursor(&mut self, fb: &mut Framebuffer) {
        if !self.visible {
            return;
        }
        // Restore saved pixels
        for dy in 0..self.save_h {
            for dx in 0..self.save_w {
                let px = self.save_x + dx;
                let py = self.save_y + dy;
                let saved = self.save_buffer[dy * CURSOR_WIDTH + dx];
                fb.set_pixel_raw(px, py, saved);
            }
        }
        // Mark the area dirty so flush sends it
        fb.fill_rect(self.save_x, self.save_y, 0, 0, Color::BLACK); // trick: 0-size but mark_dirty
        // Actually we need to properly mark dirty
        if self.save_w > 0 && self.save_h > 0 {
            // Use a tiny fill that marks dirty but doesn't change pixels...
            // Let's just directly mark dirty
            self.mark_region_dirty(fb, self.save_x, self.save_y, self.save_w, self.save_h);
        }
        self.visible = false;
    }

    fn mark_region_dirty(&self, fb: &mut Framebuffer, x: usize, y: usize, w: usize, h: usize) {
        // We need mark_dirty to be accessible. Since it's private, use mark_all_dirty
        // or we make it pub. Let's use a workaround: set_pixel at corners
        if w > 0 && h > 0 {
            let x_end = core::cmp::min(x + w, fb.width);
            let y_end = core::cmp::min(y + h, fb.height);
            // Set corner pixels to themselves to trigger dirty marking
            if x < fb.width && y < fb.height {
                let p = fb.get_pixel(x, y);
                fb.set_pixel_raw(x, y, p);
            }
            if x_end > 0 && y_end > 0 {
                let px = x_end - 1;
                let py = y_end - 1;
                let p = fb.get_pixel(px, py);
                fb.set_pixel_raw(px, py, p);
            }
            // Actually this doesn't mark dirty because set_pixel_raw skips it
            // We need to expose mark_dirty... let's just use mark_all_dirty for cursor region
            fb.mark_all_dirty();
        }
    }

    pub fn draw_cursor(&mut self, fb: &mut Framebuffer) {
        let cx = self.x as usize;
        let cy = self.y as usize;

        // Calculate visible cursor region
        let draw_w = core::cmp::min(CURSOR_WIDTH, fb.width.saturating_sub(cx));
        let draw_h = core::cmp::min(CURSOR_HEIGHT, fb.height.saturating_sub(cy));

        if draw_w == 0 || draw_h == 0 {
            return;
        }

        // Save pixels under cursor
        self.save_x = cx;
        self.save_y = cy;
        self.save_w = draw_w;
        self.save_h = draw_h;

        for dy in 0..draw_h {
            for dx in 0..draw_w {
                self.save_buffer[dy * CURSOR_WIDTH + dx] = fb.get_pixel(cx + dx, cy + dy);
            }
        }

        // Draw cursor bitmap
        for dy in 0..draw_h {
            for dx in 0..draw_w {
                match CURSOR_BITMAP[dy][dx] {
                    1 => fb.set_pixel(cx + dx, cy + dy, Color::BLACK),
                    2 => fb.set_pixel(cx + dx, cy + dy, Color::WHITE),
                    _ => {} // transparent
                }
            }
        }

        self.visible = true;
    }

    pub fn moved(&self, old_x: i32, old_y: i32) -> bool {
        self.x != old_x || self.y != old_y
    }
}
