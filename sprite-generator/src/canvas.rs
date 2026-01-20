use rust_gfx::color::{blend_colors, Rgba8888};
use rust_gfx::types::{Area, Opa, OPA_COVER};
use rust_gfx::Rasterizer;

pub struct Canvas {
    pub width: usize,
    pub height: usize,
    buffer: Vec<Rgba8888>,
}

impl Canvas {
    pub fn new(width: usize, height: usize) -> Self {
        Self {
            width,
            height,
            buffer: vec![Rgba8888::TRANSPARENT; width * height],
        }
    }

    pub fn clear(&mut self, color: Rgba8888) {
        self.buffer.fill(color);
    }

    #[inline]
    pub fn get_pixel(&self, x: i32, y: i32) -> Rgba8888 {
        if x < 0 || y < 0 || x >= self.width as i32 || y >= self.height as i32 {
            return Rgba8888::TRANSPARENT;
        }
        self.buffer[y as usize * self.width + x as usize]
    }

    #[inline]
    pub fn set_pixel(&mut self, x: i32, y: i32, color: Rgba8888) {
        if x < 0 || y < 0 || x >= self.width as i32 || y >= self.height as i32 {
            return;
        }
        self.buffer[y as usize * self.width + x as usize] = color;
    }

    #[inline]
    pub fn blend_pixel(&mut self, x: i32, y: i32, color: Rgba8888, opa: Opa) {
        self.blend_pixel_internal(x, y, color, opa);
    }

    #[inline]
    fn blend_pixel_internal(&mut self, x: i32, y: i32, color: Rgba8888, opa: Opa) {
        if x < 0 || y < 0 || x >= self.width as i32 || y >= self.height as i32 {
            return;
        }

        let idx = y as usize * self.width + x as usize;
        let bg = self.buffer[idx];
        self.buffer[idx] = blend_colors(bg, color, opa);
    }

    pub fn fill_area(&mut self, area: &Area, color: Rgba8888, opa: Opa) {
        if opa == 0 {
            return;
        }

        let x1 = area.x1.max(0);
        let y1 = area.y1.max(0);
        let x2 = area.x2.min(self.width as i32 - 1);
        let y2 = area.y2.min(self.height as i32 - 1);

        if x1 > x2 || y1 > y2 {
            return;
        }

        for y in y1..=y2 {
            for x in x1..=x2 {
                self.blend_pixel_internal(x, y, color, opa);
            }
        }
    }

    pub fn buffer(&self) -> &[Rgba8888] {
        &self.buffer
    }

    pub fn buffer_mut(&mut self) -> &mut [Rgba8888] {
        &mut self.buffer
    }
}

impl Rasterizer for Canvas {
    fn width(&self) -> usize {
        self.width
    }

    fn height(&self) -> usize {
        self.height
    }

    fn buffer_mut(&mut self) -> &mut [u8] {
        unsafe {
            std::slice::from_raw_parts_mut(
                self.buffer.as_mut_ptr() as *mut u8,
                self.buffer.len() * 4,
            )
        }
    }

    fn clear(&mut self, color: Rgba8888) {
        Canvas::clear(self, color);
    }

    fn blend_pixel(&mut self, x: i32, y: i32, color: Rgba8888, coverage: u8) {
        self.blend_pixel_internal(x, y, color, coverage);
    }

    fn fill_rect(&mut self, x: i32, y: i32, w: i32, h: i32, color: Rgba8888) {
        let area = Area {
            x1: x,
            y1: y,
            x2: x + w - 1,
            y2: y + h - 1,
        };
        self.fill_area(&area, color, OPA_COVER);
    }

    fn stamp_rgb_zero_alpha(&mut self, x: i32, y: i32, color: Rgba8888) {
        if x < 0 || y < 0 || x >= self.width as i32 || y >= self.height as i32 {
            return;
        }
        let idx = y as usize * self.width + x as usize;
        self.buffer[idx] = color.with_alpha(0);
    }
}
