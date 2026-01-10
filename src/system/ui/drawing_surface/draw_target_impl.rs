//! DrawTarget implementation for DrawingSurface
//! 
//! This allows DrawingSurface to work with the new Layer-based rendering system.

use super::surface::DrawingSurface;
use crate::libs::gfx::core::{blend::{BlendDescriptor, BlendMode, BlendSource}, Color, ColorFormat, Opacity};
use crate::libs::gfx::draw_target::DrawTarget;

impl DrawTarget for DrawingSurface<'_> {
    fn color_format(&self) -> ColorFormat {
        use crate::system::hal::display::PixelFormat;
        
        match self.pixel_format() {
            PixelFormat::Rgb565 => ColorFormat::Rgb565,
            PixelFormat::Gray4 => ColorFormat::L8, // Approximate
            _ => ColorFormat::Rgb565, // Default fallback
        }
    }
    
    fn dimensions(&self) -> (u32, u32) {
        (self.width(), self.height())
    }
    
    fn buffer_mut(&mut self) -> &mut [u8] {
        self.buffer_mut()
    }
    
    fn buffer(&self) -> &[u8] {
        self.buffer()
    }
    
    fn blend(&mut self, desc: &BlendDescriptor) {
        // Dispatch based on source type
        match &desc.source {
            Some(BlendSource::SolidColor { color }) => {
                self.blend_fill(desc.dest_rect, *color, desc.opacity, desc.mask.as_deref(), desc.mode);
            }
            Some(BlendSource::Image { buffer, width, height, format }) => {
                self.blend_image(
                    desc.dest_rect,
                    buffer,
                    *format,
                    *width as usize * 2, // stride (assuming RGB565 for now)
                    desc.opacity,
                    desc.mask.as_deref(),
                    desc.mode,
                );
            }
            Some(BlendSource::HorizontalSpan { colors }) => {
                // Handle gradients - not implemented yet
                let _ = colors;
            }
            None => {
                // Clear operation
                self.blend_fill(
                    desc.dest_rect,
                    crate::libs::gfx::core::ColorAlpha::rgba(0, 0, 0, 0),
                    desc.opacity,
                    desc.mask.as_deref(),
                    desc.mode,
                );
            }
        }
        
        // Mark the area as dirty so compositor knows to update
        self.mark_dirty(desc.dest_rect.into());
    }
    
    fn clear(&mut self, color: Color) {
        // Convert to Rgba8888 and use existing clear (which marks dirty)
        let rgba = crate::libs::gfx::color::Rgba8888::rgba(color.r, color.g, color.b, 255);
        self.clear(rgba);
    }
}

// Format-specific blend implementations
impl DrawingSurface<'_> {
    /// Blend a solid color fill
    fn blend_fill(
        &mut self,
        area: crate::libs::gfx::core::Rect,
        color: crate::libs::gfx::core::ColorAlpha,
        opacity: Opacity,
        mask: Option<&[u8]>,
        blend_mode: BlendMode,
    ) {
        use crate::system::hal::display::PixelFormat;
        
        // Fast path: opaque fill with no mask and normal blend mode
        if opacity.is_opaque() && mask.is_none() && blend_mode == BlendMode::Normal && color.a == 255 {
            match self.pixel_format() {
                PixelFormat::Rgb565 => {
                    self.blend_fill_rgb565_fast(area, color);
                }
                _ => {
                    // Fallback to slow path
                    self.blend_fill_slow(area, color, opacity, mask, blend_mode);
                }
            }
        } else {
            // Slow path: with blending
            self.blend_fill_slow(area, color, opacity, mask, blend_mode);
        }
    }
    
    /// Fast opaque RGB565 fill (no blending)
    fn blend_fill_rgb565_fast(&mut self, area: crate::libs::gfx::core::Rect, color: crate::libs::gfx::core::ColorAlpha) {
        let width = self.width() as i32;
        let height = self.height() as i32;
        let buffer = self.buffer_mut();
        
        // Convert color to RGB565
        let rgb565 = ((color.r as u16 & 0xF8) << 8)
            | ((color.g as u16 & 0xFC) << 3)
            | (color.b as u16 >> 3);
        
        let color_bytes = rgb565.to_le_bytes();
        
        // Fill row by row
        for y in area.y..(area.y + area.height as i32) {
            if y < 0 || y >= height {
                continue;
            }
            
            for x in area.x..(area.x + area.width as i32) {
                if x < 0 || x >= width {
                    continue;
                }
                
                let offset = (y * width + x) as usize * 2;
                if offset + 1 < buffer.len() {
                    buffer[offset] = color_bytes[0];
                    buffer[offset + 1] = color_bytes[1];
                }
            }
        }
    }
    
    /// Slow path: with alpha blending
    fn blend_fill_slow(
        &mut self,
        area: crate::libs::gfx::core::Rect,
        color: crate::libs::gfx::core::ColorAlpha,
        opacity: Opacity,
        mask: Option<&[u8]>,
        _blend_mode: BlendMode,
    ) {
        use crate::system::hal::display::PixelFormat;
        
        let width = self.width() as i32;
        let height = self.height() as i32;
        let pixel_format = self.pixel_format();
        let buffer = self.buffer_mut();
        
        // Calculate effective opacity
        let base_opa = ((color.a as u16 * opacity.value() as u16) / 255) as u8;
        
        match pixel_format {
            PixelFormat::Rgb565 => {
                for y in area.y..(area.y + area.height as i32) {
                    if y < 0 || y >= height {
                        continue;
                    }
                    
                    for x in area.x..(area.x + area.width as i32) {
                        if x < 0 || x >= width {
                            continue;
                        }
                        
                        // Calculate final opacity with mask
                        let final_opa = if let Some(mask_buf) = mask {
                            let mask_x = (x - area.x) as usize;
                            let mask_y = (y - area.y) as usize;
                            let mask_idx = mask_y * area.width as usize + mask_x;
                            if mask_idx < mask_buf.len() {
                                ((base_opa as u16 * mask_buf[mask_idx] as u16) / 255) as u8
                            } else {
                                base_opa
                            }
                        } else {
                            base_opa
                        };
                        
                        if final_opa == 0 {
                            continue; // Fully transparent
                        }
                        
                        let offset = (y * width + x) as usize * 2;
                        if offset + 1 < buffer.len() {
                            if final_opa == 255 {
                                // Opaque - no blending needed
                                let rgb565 = ((color.r as u16 & 0xF8) << 8)
                                    | ((color.g as u16 & 0xFC) << 3)
                                    | (color.b as u16 >> 3);
                                let bytes = rgb565.to_le_bytes();
                                buffer[offset] = bytes[0];
                                buffer[offset + 1] = bytes[1];
                            } else {
                                // Alpha blending
                                let bg_rgb565 = u16::from_le_bytes([buffer[offset], buffer[offset + 1]]);
                                
                                // Extract background RGB565 components
                                let bg_r = ((bg_rgb565 >> 11) & 0x1F) as u8;
                                let bg_g = ((bg_rgb565 >> 5) & 0x3F) as u8;
                                let bg_b = (bg_rgb565 & 0x1F) as u8;
                                
                                // Convert to 8-bit
                                let bg_r8 = (bg_r << 3) | (bg_r >> 2);
                                let bg_g8 = (bg_g << 2) | (bg_g >> 4);
                                let bg_b8 = (bg_b << 3) | (bg_b >> 2);
                                
                                // Blend
                                let inv_opa = 255 - final_opa;
                                let out_r = ((color.r as u16 * final_opa as u16
                                    + bg_r8 as u16 * inv_opa as u16)
                                    / 255) as u8;
                                let out_g = ((color.g as u16 * final_opa as u16
                                    + bg_g8 as u16 * inv_opa as u16)
                                    / 255) as u8;
                                let out_b = ((color.b as u16 * final_opa as u16
                                    + bg_b8 as u16 * inv_opa as u16)
                                    / 255) as u8;
                                
                                // Convert back to RGB565
                                let out_rgb565 = ((out_r as u16 & 0xF8) << 8)
                                    | ((out_g as u16 & 0xFC) << 3)
                                    | (out_b as u16 >> 3);
                                let bytes = out_rgb565.to_le_bytes();
                                buffer[offset] = bytes[0];
                                buffer[offset + 1] = bytes[1];
                            }
                        }
                    }
                }
            }
            _ => {
                // Other formats not yet implemented
            }
        }
    }
    
    /// Blend from source image
    fn blend_image(
        &mut self,
        _area: crate::libs::gfx::core::Rect,
        _source: &[u8],
        _source_format: ColorFormat,
        _source_stride: usize,
        _opacity: Opacity,
        _mask: Option<&[u8]>,
        _blend_mode: BlendMode,
    ) {
        // TODO: Implement image blending
        // Not critical for current use case (primitives mostly use fill)
    }
}
