extern crate alloc;

use alloc::vec;
use alloc::vec::Vec;
use uefi::boot;
use uefi::proto::console::gop::{BltOp, BltPixel, BltRegion, GraphicsOutput};

#[derive(Clone, Copy)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl Color {
    pub const fn new(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b }
    }

    pub const fn to_blt(self) -> BltPixel {
        BltPixel::new(self.r, self.g, self.b)
    }

    pub const BLACK: Self = Self::new(0, 0, 0);
    pub const WHITE: Self = Self::new(255, 255, 255);
    pub const TERMINAL_BG: Self = Self::new(12, 12, 12);
    pub const TASKBAR: Self = Self::new(45, 45, 48);
    pub const GREEN: Self = Self::new(80, 220, 80);
    pub const RED: Self = Self::new(255, 80, 80);
    pub const BRIGHT_RED: Self = Self::new(232, 17, 35);
    pub const CYAN: Self = Self::new(100, 200, 255);
    pub const YELLOW: Self = Self::new(255, 255, 100);
    pub const LIGHT_GRAY: Self = Self::new(200, 200, 200);
    pub const TITLE_BAR: Self = Self::new(50, 50, 55);
    pub const MENU_BG: Self = Self::new(40, 40, 45);
    pub const MENU_HOVER: Self = Self::new(55, 55, 60);
    pub const TASKBAR_HOVER: Self = Self::new(60, 60, 65);
}

#[derive(Clone, Copy)]
pub struct ScreenInfo {
    pub width: usize,
    pub height: usize,
}

pub fn init_gop() -> Result<ScreenInfo, &'static str> {
    let handle = boot::get_handle_for_protocol::<GraphicsOutput>()
        .map_err(|_| "No GOP handle")?;
    let mut gop = boot::open_protocol_exclusive::<GraphicsOutput>(handle)
        .map_err(|_| "Failed to open GOP")?;

    let best_mode = gop
        .modes()
        .filter(|m| {
            let (w, h) = m.info().resolution();
            w <= 1280 && h <= 1024
        })
        .max_by_key(|m| {
            let (w, h) = m.info().resolution();
            w * h
        });

    if let Some(mode) = best_mode {
        let _ = gop.set_mode(&mode);
    }

    let info = gop.current_mode_info();
    let (width, height) = info.resolution();

    Ok(ScreenInfo { width, height })
}

pub struct Framebuffer {
    pub pixels: Vec<BltPixel>,
    pub width: usize,
    pub height: usize,
    dirty_x_min: usize,
    dirty_y_min: usize,
    dirty_x_max: usize,
    dirty_y_max: usize,
}

impl Framebuffer {
    pub fn new(width: usize, height: usize) -> Self {
        Self {
            pixels: vec![BltPixel::new(0, 0, 0); width * height],
            width,
            height,
            // Start fully dirty so first flush draws everything
            dirty_x_min: 0,
            dirty_y_min: 0,
            dirty_x_max: width,
            dirty_y_max: height,
        }
    }

    fn mark_dirty(&mut self, x: usize, y: usize, w: usize, h: usize) {
        if w == 0 || h == 0 {
            return;
        }
        let x_end = core::cmp::min(x + w, self.width);
        let y_end = core::cmp::min(y + h, self.height);
        if x < self.dirty_x_min {
            self.dirty_x_min = x;
        }
        if y < self.dirty_y_min {
            self.dirty_y_min = y;
        }
        if x_end > self.dirty_x_max {
            self.dirty_x_max = x_end;
        }
        if y_end > self.dirty_y_max {
            self.dirty_y_max = y_end;
        }
    }

    /// Mark the entire framebuffer as dirty (for full redraws)
    pub fn mark_all_dirty(&mut self) {
        self.dirty_x_min = 0;
        self.dirty_y_min = 0;
        self.dirty_x_max = self.width;
        self.dirty_y_max = self.height;
    }

    pub fn set_pixel(&mut self, x: usize, y: usize, color: Color) {
        if x < self.width && y < self.height {
            self.pixels[y * self.width + x] = color.to_blt();
            self.mark_dirty(x, y, 1, 1);
        }
    }

    /// Set pixel without dirty tracking (for background cache restore)
    pub fn set_pixel_raw(&mut self, x: usize, y: usize, pixel: BltPixel) {
        if x < self.width && y < self.height {
            self.pixels[y * self.width + x] = pixel;
        }
    }

    pub fn get_pixel(&self, x: usize, y: usize) -> BltPixel {
        if x < self.width && y < self.height {
            self.pixels[y * self.width + x]
        } else {
            BltPixel::new(0, 0, 0)
        }
    }

    pub fn fill_rect(&mut self, x: usize, y: usize, w: usize, h: usize, color: Color) {
        let pixel = color.to_blt();
        let x_end = core::cmp::min(x + w, self.width);
        let y_end = core::cmp::min(y + h, self.height);
        for row in y..y_end {
            let start = row * self.width + x;
            let end = row * self.width + x_end;
            for p in &mut self.pixels[start..end] {
                *p = pixel;
            }
        }
        self.mark_dirty(x, y, w, h);
    }

    pub fn flush(&mut self) {
        if self.dirty_x_min >= self.dirty_x_max || self.dirty_y_min >= self.dirty_y_max {
            return;
        }

        let Ok(handle) = boot::get_handle_for_protocol::<GraphicsOutput>() else {
            return;
        };
        let Ok(mut gop) = boot::open_protocol_exclusive::<GraphicsOutput>(handle) else {
            return;
        };

        let dx = self.dirty_x_min;
        let dy = self.dirty_y_min;
        let w = self.dirty_x_max - dx;
        let h = self.dirty_y_max - dy;

        let _ = gop.blt(BltOp::BufferToVideo {
            buffer: &self.pixels,
            src: BltRegion::SubRectangle {
                coords: (dx, dy),
                px_stride: self.width,
            },
            dest: (dx, dy),
            dims: (w, h),
        });

        // Reset dirty rect
        self.dirty_x_min = self.width;
        self.dirty_y_min = self.height;
        self.dirty_x_max = 0;
        self.dirty_y_max = 0;
    }
}
