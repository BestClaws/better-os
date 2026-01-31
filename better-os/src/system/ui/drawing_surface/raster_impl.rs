
use super::surface::{
    gray4_to_rgba8888, rgb565_to_rgba8888, rgba8888_to_gray4, rgba8888_to_rgb565, DrawingSurface,
};
use crate::system::hal::display::PixelFormat;
use crate::util::math::primitives::Rect;
use gfx::colors::{blend_colors, Color};
use gfx::rasterizer::RasterTarget;

impl<'a> RasterTarget for DrawingSurface<'a> {
    fn width(&self) -> usize {
        self.width() as usize
    }

    fn height(&self) -> usize {
        self.height() as usize
    }

    fn buffer_mut(&mut self) -> &mut [u8] {
        self.buffer_mut()
    }



    fn clear(&mut self, color: Color) {
        self.clear(color);
    }

    fn blend_pixel(&mut self, x: i32, y: i32, color: Color, coverage: u8) {
        if coverage == 0 {
            return;
        }
        if !self.is_attached() {
            return;
        }
        let Some(index) = self.pixel_index(x, y) else {
            return;
        };

        match self.pixel_format() {
            PixelFormat::Rgb565 => {
                let byte_index = index * 2;
                {
                    let buf = self.buffer_mut();
                    if byte_index + 1 >= buf.len() {
                        return;
                    }
                    let current = u16::from_be_bytes([buf[byte_index], buf[byte_index + 1]]);
                    let encoded = if coverage == 255 {
                        rgba8888_to_rgb565(color)
                    } else {
                        let dst = rgb565_to_rgba8888(current);
                        let mixed = blend_colors(dst, color, coverage);
                        rgba8888_to_rgb565(mixed)
                    };
                    buf[byte_index] = (encoded >> 8) as u8;
                    buf[byte_index + 1] = (encoded & 0xFF) as u8;
                }
            }
            PixelFormat::Gray4 => {
                let byte_index = index / 2;
                {
                    let buf = self.buffer_mut();
                    if byte_index >= buf.len() {
                        return;
                    }
                    let high = (index & 1) == 0;
                    let current_byte = buf[byte_index];
                    let current_gray = if high {
                        (current_byte >> 4) & 0x0F
                    } else {
                        current_byte & 0x0F
                    };

                    let encoded = if coverage == 255 {
                        rgba8888_to_gray4(color)
                    } else {
                        let dst = gray4_to_rgba8888(current_gray);
                        let mixed = blend_colors(dst, color, coverage);
                        rgba8888_to_gray4(mixed)
                    } & 0x0F;

                    if high {
                        buf[byte_index] = (current_byte & 0x0F) | (encoded << 4);
                    } else {
                        buf[byte_index] = (current_byte & 0xF0) | encoded;
                    }
                }
            }
        }

    }

    fn blend_hspan_with(
        &mut self,
        x: i32,
        y: i32,
        len: i32,
        mut f: impl FnMut(usize) -> (Color, u8),
    ) {
        if len <= 0 {
            return;
        }
        if y < 0 || y >= self.height() as i32 {
            return;
        }
        if !self.is_attached() {
            return;
        }

        let start_x = x.max(0);
        let end_x = (x + len).min(self.width() as i32);
        if start_x >= end_x {
            return;
        }

        let skip = (start_x - x) as usize;
        let run_len = (end_x - start_x) as usize;

        match self.pixel_format() {
            PixelFormat::Rgb565 => {
                let stride_bytes = self.width() as usize * 2;
                let mut byte_index = y as usize * stride_bytes + start_x as usize * 2;
                let buf = self.buffer_mut();
                for i in 0..run_len {
                    let (color, coverage) = f(i + skip);
                    if coverage == 0 {
                        byte_index += 2;
                        continue;
                    }
                    let current = u16::from_be_bytes([buf[byte_index], buf[byte_index + 1]]);
                    let encoded = if coverage == 255 {
                        rgba8888_to_rgb565(color)
                    } else {
                        let dst = rgb565_to_rgba8888(current);
                        let mixed = blend_colors(dst, color, coverage);
                        rgba8888_to_rgb565(mixed)
                    };
                    buf[byte_index] = (encoded >> 8) as u8;
                    buf[byte_index + 1] = (encoded & 0xFF) as u8;
                    byte_index += 2;
                }
            }
            PixelFormat::Gray4 => {
                let stride = self.width() as usize;
                let base = y as usize * stride;
                let buf = self.buffer_mut();
                for i in 0..run_len {
                    let (color, coverage) = f(i + skip);
                    let pixel_index = base + start_x as usize + i;
                    let byte_index = pixel_index / 2;
                    let high = (pixel_index & 1) == 0;
                    let current_byte = buf[byte_index];
                    let current_gray = if high {
                        (current_byte >> 4) & 0x0F
                    } else {
                        current_byte & 0x0F
                    };

                    let encoded = if coverage == 255 {
                        rgba8888_to_gray4(color)
                    } else if coverage == 0 {
                        current_gray
                    } else {
                        let dst = gray4_to_rgba8888(current_gray);
                        let mixed = blend_colors(dst, color, coverage);
                        rgba8888_to_gray4(mixed)
                    } & 0x0F;

                    if high {
                        buf[byte_index] = (current_byte & 0x0F) | (encoded << 4);
                    } else {
                        buf[byte_index] = (current_byte & 0xF0) | encoded;
                    }
                }
            }
        }


    }

    fn blend_vspan_with(
        &mut self,
        x: i32,
        y: i32,
        len: i32,
        mut f: impl FnMut(usize) -> (Color, u8),
    ) {
        if len <= 0 {
            return;
        }
        if x < 0 || x >= self.width() as i32 {
            return;
        }
        if !self.is_attached() {
            return;
        }

        let start_y = y.max(0);
        let end_y = (y + len).min(self.height() as i32);
        if start_y >= end_y {
            return;
        }

        let skip = (start_y - y) as usize;
        let run_len = (end_y - start_y) as usize;
        for i in 0..run_len {
            let (color, coverage) = f(i + skip);
            self.blend_pixel(x, start_y + i as i32, color, coverage);
        }

    }

    fn fill_rect(&mut self, x: i32, y: i32, w: i32, h: i32, color: Color) {
        if w <= 0 || h <= 0 {
            return;
        }
        if !self.is_attached() {
            return;
        }

        let width = self.width() as i32;
        let height = self.height() as i32;
        let x0 = x.max(0);
        let y0 = y.max(0);
        let x1 = (x + w).min(width);
        let y1 = (y + h).min(height);
        if x0 >= x1 || y0 >= y1 {
            return;
        }

        match self.pixel_format() {
            PixelFormat::Rgb565 => {
                let encoded = rgba8888_to_rgb565(color);
                let hi = (encoded >> 8) as u8;
                let lo = (encoded & 0xFF) as u8;
                let stride = self.width() as usize;
                let buf = self.buffer_mut();
                for row in y0..y1 {
                    let mut byte_index = (row as usize * stride + x0 as usize) * 2;
                    for _ in x0..x1 {
                        buf[byte_index] = hi;
                        buf[byte_index + 1] = lo;
                        byte_index += 2;
                    }
                }
            }
            PixelFormat::Gray4 => {
                let gray = rgba8888_to_gray4(color) & 0x0F;
                let stride = self.width() as usize;
                let buf = self.buffer_mut();
                for row in y0..y1 {
                    for col in x0..x1 {
                        let pixel_index = row as usize * stride + col as usize;
                        let byte_index = pixel_index / 2;
                        let high = (pixel_index & 1) == 0;
                        if high {
                            buf[byte_index] = (buf[byte_index] & 0x0F) | (gray << 4);
                        } else {
                            buf[byte_index] = (buf[byte_index] & 0xF0) | gray;
                        }
                    }
                }
            }
        }


    }

    fn stamp_rgb_zero_alpha(&mut self, x: i32, y: i32, color: Color) {
        match self.pixel_format() {
            PixelFormat::Rgb565 | PixelFormat::Gray4 => {
                let _ = (x, y, color);
                // Formats without alpha storage should leave the existing pixel untouched.
            }
        }
    }
}
