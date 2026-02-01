//! RGB565 pixel format rasterizer

use crate::colors::Color;
use crate::rasterizer::RasterTarget;

/// RGB565 rasterizer that wraps a framebuffer
pub struct Rgb565Rasterizer<'a> {
    buffer: &'a mut [u8],
    width: u16,
    height: u16,
}

impl<'a> Rgb565Rasterizer<'a> {
    /// Create a new RGB565 rasterizer wrapping a framebuffer
    pub fn new(buffer: &'a mut [u8], width: u16, height: u16) -> Self {
        Self { buffer, width, height }
    }
}

impl<'a> RasterTarget for Rgb565Rasterizer<'a> {
    fn width(&self) -> u16 {
        self.width
    }

    fn height(&self) -> u16 {
        self.height
    }

    fn fill_solid_hspan(&mut self, y: u16, x_start: u16, color: Color, length: u16) {
        let rgb565 = rgb888_to_rgb565(color.r(), color.g(), color.b());
        let high = (rgb565 >> 8) as u8;
        let low = (rgb565 & 0xFF) as u8;
        
        for x in x_start..x_start.saturating_add(length) {
            if x < self.width && y < self.height {
                let idx = ((y as usize * self.width as usize) + x as usize) * 2;
                self.buffer[idx] = high;
                self.buffer[idx + 1] = low;
            }
        }
    }

    fn blend_solid_hspan(&mut self, y: u16, x_start: u16, color: Color, coverage: &[u8]) {
        let fg_565 = rgb888_to_rgb565(color.r(), color.g(), color.b());
        let fg_r = (fg_565 >> 11) & 0x1F;
        let fg_g = (fg_565 >> 5) & 0x3F;
        let fg_b = fg_565 & 0x1F;
        
        for (i, &alpha) in coverage.iter().enumerate() {
            let x = x_start + i as u16;
            if x < self.width && y < self.height && alpha > 0 {
                let idx = ((y as usize * self.width as usize) + x as usize) * 2;
                
                if alpha == 255 {
                    // Fast path: fully opaque
                    self.buffer[idx] = (fg_565 >> 8) as u8;
                    self.buffer[idx + 1] = (fg_565 & 0xFF) as u8;
                } else {
                    // Alpha blend
                    let bg_high = self.buffer[idx];
                    let bg_low = self.buffer[idx + 1];
                    let bg_565 = ((bg_high as u16) << 8) | (bg_low as u16);
                    
                    let bg_r = (bg_565 >> 11) & 0x1F;
                    let bg_g = (bg_565 >> 5) & 0x3F;
                    let bg_b = bg_565 & 0x1F;
                    
                    let alpha_norm = alpha as u16;
                    let inv_alpha = 255 - alpha_norm;
                    
                    let out_r = ((fg_r * alpha_norm + bg_r * inv_alpha) / 255) & 0x1F;
                    let out_g = ((fg_g * alpha_norm + bg_g * inv_alpha) / 255) & 0x3F;
                    let out_b = ((fg_b * alpha_norm + bg_b * inv_alpha) / 255) & 0x1F;
                    
                    let out_565 = (out_r << 11) | (out_g << 5) | out_b;
                    self.buffer[idx] = (out_565 >> 8) as u8;
                    self.buffer[idx + 1] = (out_565 & 0xFF) as u8;
                }
            }
        }
    }

    fn blend_color_hspan(&mut self, y: u16, x_start: u16, colors: &[Color], coverage: &[u8]) {
        for (i, (&color, &alpha)) in colors.iter().zip(coverage.iter()).enumerate() {
            let x = x_start + i as u16;
            if x < self.width && y < self.height && alpha > 0 {
                let idx = ((y as usize * self.width as usize) + x as usize) * 2;
                let fg_565 = rgb888_to_rgb565(color.r(), color.g(), color.b());
                
                if alpha == 255 {
                    // Fast path: fully opaque
                    self.buffer[idx] = (fg_565 >> 8) as u8;
                    self.buffer[idx + 1] = (fg_565 & 0xFF) as u8;
                } else {
                    // Alpha blend
                    let bg_high = self.buffer[idx];
                    let bg_low = self.buffer[idx + 1];
                    let bg_565 = ((bg_high as u16) << 8) | (bg_low as u16);
                    
                    let fg_r = (fg_565 >> 11) & 0x1F;
                    let fg_g = (fg_565 >> 5) & 0x3F;
                    let fg_b = fg_565 & 0x1F;
                    
                    let bg_r = (bg_565 >> 11) & 0x1F;
                    let bg_g = (bg_565 >> 5) & 0x3F;
                    let bg_b = bg_565 & 0x1F;
                    
                    let alpha_norm = alpha as u16;
                    let inv_alpha = 255 - alpha_norm;
                    
                    let out_r = ((fg_r * alpha_norm + bg_r * inv_alpha) / 255) & 0x1F;
                    let out_g = ((fg_g * alpha_norm + bg_g * inv_alpha) / 255) & 0x3F;
                    let out_b = ((fg_b * alpha_norm + bg_b * inv_alpha) / 255) & 0x1F;
                    
                    let out_565 = (out_r << 11) | (out_g << 5) | out_b;
                    self.buffer[idx] = (out_565 >> 8) as u8;
                    self.buffer[idx + 1] = (out_565 & 0xFF) as u8;
                }
            }
        }
    }

    fn fill_solid_vspan(&mut self, x: u16, y_start: u16, color: Color, length: u16) {
        let rgb565 = rgb888_to_rgb565(color.r(), color.g(), color.b());
        let high = (rgb565 >> 8) as u8;
        let low = (rgb565 & 0xFF) as u8;
        
        for y in y_start..y_start.saturating_add(length) {
            if x < self.width && y < self.height {
                let idx = ((y as usize * self.width as usize) + x as usize) * 2;
                self.buffer[idx] = high;
                self.buffer[idx + 1] = low;
            }
        }
    }

    fn blend_solid_vspan(&mut self, x: u16, y_start: u16, color: Color, coverage: &[u8]) {
        let fg_565 = rgb888_to_rgb565(color.r(), color.g(), color.b());
        let fg_r = (fg_565 >> 11) & 0x1F;
        let fg_g = (fg_565 >> 5) & 0x3F;
        let fg_b = fg_565 & 0x1F;
        
        for (i, &alpha) in coverage.iter().enumerate() {
            let y = y_start + i as u16;
            if x < self.width && y < self.height && alpha > 0 {
                let idx = ((y as usize * self.width as usize) + x as usize) * 2;
                
                if alpha == 255 {
                    // Fast path: fully opaque
                    self.buffer[idx] = (fg_565 >> 8) as u8;
                    self.buffer[idx + 1] = (fg_565 & 0xFF) as u8;
                } else {
                    // Alpha blend
                    let bg_high = self.buffer[idx];
                    let bg_low = self.buffer[idx + 1];
                    let bg_565 = ((bg_high as u16) << 8) | (bg_low as u16);
                    
                    let bg_r = (bg_565 >> 11) & 0x1F;
                    let bg_g = (bg_565 >> 5) & 0x3F;
                    let bg_b = bg_565 & 0x1F;
                    
                    let alpha_norm = alpha as u16;
                    let inv_alpha = 255 - alpha_norm;
                    
                    let out_r = ((fg_r * alpha_norm + bg_r * inv_alpha) / 255) & 0x1F;
                    let out_g = ((fg_g * alpha_norm + bg_g * inv_alpha) / 255) & 0x3F;
                    let out_b = ((fg_b * alpha_norm + bg_b * inv_alpha) / 255) & 0x1F;
                    
                    let out_565 = (out_r << 11) | (out_g << 5) | out_b;
                    self.buffer[idx] = (out_565 >> 8) as u8;
                    self.buffer[idx + 1] = (out_565 & 0xFF) as u8;
                }
            }
        }
    }

    fn blend_color_vspan(&mut self, x: u16, y_start: u16, colors: &[Color], coverage: &[u8]) {
        for (i, (&color, &alpha)) in colors.iter().zip(coverage.iter()).enumerate() {
            let y = y_start + i as u16;
            if x < self.width && y < self.height && alpha > 0 {
                let idx = ((y as usize * self.width as usize) + x as usize) * 2;
                let fg_565 = rgb888_to_rgb565(color.r(), color.g(), color.b());
                
                if alpha == 255 {
                    // Fast path: fully opaque
                    self.buffer[idx] = (fg_565 >> 8) as u8;
                    self.buffer[idx + 1] = (fg_565 & 0xFF) as u8;
                } else {
                    // Alpha blend
                    let bg_high = self.buffer[idx];
                    let bg_low = self.buffer[idx + 1];
                    let bg_565 = ((bg_high as u16) << 8) | (bg_low as u16);
                    
                    let fg_r = (fg_565 >> 11) & 0x1F;
                    let fg_g = (fg_565 >> 5) & 0x3F;
                    let fg_b = fg_565 & 0x1F;
                    
                    let bg_r = (bg_565 >> 11) & 0x1F;
                    let bg_g = (bg_565 >> 5) & 0x3F;
                    let bg_b = bg_565 & 0x1F;
                    
                    let alpha_norm = alpha as u16;
                    let inv_alpha = 255 - alpha_norm;
                    
                    let out_r = ((fg_r * alpha_norm + bg_r * inv_alpha) / 255) & 0x1F;
                    let out_g = ((fg_g * alpha_norm + bg_g * inv_alpha) / 255) & 0x3F;
                    let out_b = ((fg_b * alpha_norm + bg_b * inv_alpha) / 255) & 0x1F;
                    
                    let out_565 = (out_r << 11) | (out_g << 5) | out_b;
                    self.buffer[idx] = (out_565 >> 8) as u8;
                    self.buffer[idx + 1] = (out_565 & 0xFF) as u8;
                }
            }
        }
    }

    fn fill_solid_rect(&mut self, x: u16, y: u16, width: u16, height: u16, color: Color) {
        let rgb565 = rgb888_to_rgb565(color.r(), color.g(), color.b());
        let high = (rgb565 >> 8) as u8;
        let low = (rgb565 & 0xFF) as u8;
        
        for dy in 0..height {
            let py = y.saturating_add(dy);
            if py >= self.height {
                break;
            }
            
            for dx in 0..width {
                let px = x.saturating_add(dx);
                if px >= self.width {
                    break;
                }
                
                let idx = ((py as usize * self.width as usize) + px as usize) * 2;
                self.buffer[idx] = high;
                self.buffer[idx + 1] = low;
            }
        }
    }
}

/// Convert RGB888 to RGB565
#[inline]
fn rgb888_to_rgb565(r: u8, g: u8, b: u8) -> u16 {
    ((r as u16 & 0xF8) << 8) | ((g as u16 & 0xFC) << 3) | ((b as u16 & 0xF8) >> 3)
}
