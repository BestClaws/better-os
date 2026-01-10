//! High-performance format-agnostic blitter.
//! Assumes source and destination `DrawingSurface`s share identical storage format.
use defmt::debug;
use embassy_time::Instant;

use crate::system::ui::drawing_surface::DrawingSurface;
use crate::util::math::primitives::Rect;

pub struct SurfaceBlitter;

impl SurfaceBlitter {
    pub fn copy_full<'d, 's>(
        dest: &mut DrawingSurface<'d>,
        source: &DrawingSurface<'s>,
        offset_x: i32,
        offset_y: i32,
    ) {
        let timer = Instant::now();
        let (dest_w, dest_h) = (dest.width(), dest.height());
        let (src_w, src_h) = (source.width(), source.height());
        Self::copy_rows_clipped(
            dest, source, 0, 0, src_w, src_h, offset_x, offset_y, dest_w, dest_h,
        );
        debug!("Full blit: {} μs", timer.elapsed().as_micros());
    }

    pub fn copy_region<'d, 's>(
        dest: &mut DrawingSurface<'d>,
        source: &DrawingSurface<'s>,
        region: Rect,
        offset_x: i32,
        offset_y: i32,
    ) {
        let timer = Instant::now();
        let (dest_w, dest_h) = (dest.width(), dest.height());
        let (src_w, src_h) = (source.width(), source.height());
        let region_x = region.top_left.x.max(0) as u32;
        let region_y = region.top_left.y.max(0) as u32;
        let region_w = (region.top_left.x as u32 + region.size.width).min(src_w);
        let region_h = (region.top_left.y as u32 + region.size.height).min(src_h);
        if region_x >= region_w || region_y >= region_h {
            return;
        }
        Self::copy_rows_clipped(
            dest, source, region_x, region_y, region_w, region_h, offset_x, offset_y, dest_w,
            dest_h,
        );
        debug!("Region blit: {} μs", timer.elapsed().as_micros());
    }

    fn copy_rows_clipped<'d, 's>(
        dest: &mut DrawingSurface<'d>,
        source: &DrawingSurface<'s>,
        src_x0: u32,
        src_y0: u32,
        src_x1: u32,
        src_y1: u32,
        offset_x: i32,
        offset_y: i32,
        dest_w: u32,
        dest_h: u32,
    ) {
        debug_assert_eq!(dest.pixel_format(), source.pixel_format());
        let format = dest.pixel_format();
        let is_gray4 = matches!(format, crate::system::hal::display::PixelFormat::Gray4);
        
        if !is_gray4 && dest.bytes_per_pixel() == 1 {
            // Safety check: if bytes_per_pixel is 1 but not Gray8, something is wrong
            debug!("Warning: format={:?} bpp=1", format);
        }
        
        let dest_w_i = dest_w as i32;
        let dest_h_i = dest_h as i32;
        let src_w = source.width();
        
        for sy in src_y0..src_y1 {
            let dy = sy as i32 + offset_y;
            if dy < 0 || dy >= dest_h_i {
                continue;
            }
            let dx0 = src_x0 as i32 + offset_x;
            let dx1 = src_x1 as i32 + offset_x;
            let clip_x0 = dx0.max(0).min(dest_w_i);
            let clip_x1 = dx1.max(0).min(dest_w_i);
            if clip_x0 >= clip_x1 {
                continue;
            }
            let sx0 = (clip_x0 - offset_x).max(src_x0 as i32) as u32;
            let sx1 = (clip_x1 - offset_x).min(src_x1 as i32) as u32;
            if sx0 >= sx1 {
                continue;
            }
            
            if is_gray4 {
                // Gray4: 2 pixels per byte, need pixel-by-pixel copy for unaligned cases
                // For now, use pixel-by-pixel copy (can be optimized later for aligned cases)
                for i in 0..(sx1 - sx0) {
                    let src_x = sx0 + i;
                    let dst_x = clip_x0 as u32 + i;
                    let src_pixel_idx = (src_x + sy * src_w) as usize;
                    let dst_pixel_idx = (dst_x + dy as u32 * dest_w) as usize;
                    let pixel = source.get_pixel_at_index(src_pixel_idx);
                    dest.set_pixel_at_index(dst_pixel_idx, pixel);
                }
            } else {
                // Other formats: byte-aligned copy
                let bytes_per_pixel = dest.bytes_per_pixel();
                let pixels = (sx1 - sx0) as usize;
                let bytes = pixels * bytes_per_pixel;
                let src_first_pixel = (sx0 + sy * src_w) as usize;
                let dst_first_pixel = (clip_x0 as u32 + dy as u32 * dest_w) as usize;
                let src_byte = src_first_pixel * bytes_per_pixel;
                let dst_byte = dst_first_pixel * bytes_per_pixel;
                let src_slice = &source.buffer()[src_byte..src_byte + bytes];
                let dst_slice = &mut dest.buffer_mut()[dst_byte..dst_byte + bytes];
                dst_slice.copy_from_slice(src_slice);
            }
        }
    }
}
