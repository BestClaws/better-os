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
        let is_gray4 = matches!(
            dest.pixel_format(),
            crate::system::hal::display::PixelFormat::Gray4
        );

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
                let pixel_count = (sx1 - sx0) as usize;
                if pixel_count == 0 {
                    continue;
                }

                let src_pixel_start = (sx0 + sy * src_w) as usize;
                let dst_pixel_start = (clip_x0 as u32 + dy as u32 * dest_w) as usize;
                let src_buf = source.buffer();
                let dst_buf = dest.buffer_mut();
                Self::copy_gray4_span(
                    dst_buf,
                    src_buf,
                    dst_pixel_start,
                    src_pixel_start,
                    pixel_count,
                );
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

impl SurfaceBlitter {
    fn copy_gray4_span(
        dest_buf: &mut [u8],
        src_buf: &[u8],
        mut dst_pixel: usize,
        mut src_pixel: usize,
        mut count: usize,
    ) {
        if count == 0 {
            return;
        }

        if ((src_pixel ^ dst_pixel) & 1) != 0 {
            Self::copy_gray4_pixels(dest_buf, src_buf, dst_pixel, src_pixel, count);
            return;
        }

        if (src_pixel & 1) != 0 {
            Self::copy_gray4_pixels(dest_buf, src_buf, dst_pixel, src_pixel, 1);
            src_pixel += 1;
            dst_pixel += 1;
            count -= 1;
        }

        let byte_count = count / 2;
        if byte_count > 0 {
            let src_byte = src_pixel / 2;
            let dst_byte = dst_pixel / 2;
            let src_slice = &src_buf[src_byte..src_byte + byte_count];
            let dst_slice = &mut dest_buf[dst_byte..dst_byte + byte_count];
            dst_slice.copy_from_slice(src_slice);
            let advance = byte_count * 2;
            src_pixel += advance;
            dst_pixel += advance;
            count -= advance;
        }

        if count > 0 {
            Self::copy_gray4_pixels(dest_buf, src_buf, dst_pixel, src_pixel, count);
        }
    }

    fn copy_gray4_pixels(
        dest_buf: &mut [u8],
        src_buf: &[u8],
        dst_pixel: usize,
        src_pixel: usize,
        count: usize,
    ) {
        for i in 0..count {
            let src_idx = src_pixel + i;
            let dst_idx = dst_pixel + i;

            let src_byte = src_idx / 2;
            let src_nibble = if src_idx & 1 == 0 {
                (src_buf[src_byte] >> 4) & 0x0F
            } else {
                src_buf[src_byte] & 0x0F
            };

            let dst_byte = dst_idx / 2;
            let slot = &mut dest_buf[dst_byte];
            if dst_idx & 1 == 0 {
                *slot = (*slot & 0x0F) | (src_nibble << 4);
            } else {
                *slot = (*slot & 0xF0) | src_nibble;
            }
        }
    }
}
