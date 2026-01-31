use crate::system::hal::display::PixelFormat;
use crate::util::math::primitives::Rect;
use alloc::vec;
use alloc::vec::Vec as AllocVec;

pub fn extract_region_buffer(
    full_buffer: &[u8],
    region: &Rect,
    full_width: u32,
    _full_height: u32,
    bytes_per_pixel: usize,
) -> AllocVec<u8> {
    let region_x = region.top_left.x as u32;
    let region_y = region.top_left.y as u32;
    let region_width = region.size.width as u32;
    let region_height = region.size.height as u32;
    let region_size = (region_width * region_height * bytes_per_pixel as u32) as usize;
    let mut region_buffer = vec![0u8; region_size];
    let full_bytes_per_row = full_width as usize * bytes_per_pixel;
    let region_bytes_per_row = region_width as usize * bytes_per_pixel;
    for row in 0..region_height as usize {
        let src_start = ((region_y as usize + row) * full_bytes_per_row)
            + (region_x as usize * bytes_per_pixel);
        let dst_start = row * region_bytes_per_row;
        region_buffer[dst_start..dst_start + region_bytes_per_row]
            .copy_from_slice(&full_buffer[src_start..src_start + region_bytes_per_row]);
    }
    region_buffer
}

/// Extract region buffer with pixel format awareness for packed formats
pub fn extract_region_buffer_format(
    full_buffer: &[u8],
    region: &Rect,
    full_width: u32,
    full_height: u32,
    pixel_format: PixelFormat,
) -> AllocVec<u8> {
    let region_x = region.top_left.x as usize;
    let region_y = region.top_left.y as usize;
    let region_width = region.size.width as usize;
    let region_height = region.size.height as usize;

    match pixel_format {
        PixelFormat::Gray4 => {
            // Gray4: 2 pixels per byte, need pixel-level extraction
            let region_size = (region_width * region_height + 1) / 2;
            let mut region_buffer = vec![0u8; region_size];

            for row in 0..region_height {
                for col in 0..region_width {
                    let src_x = region_x + col;
                    let src_y = region_y + row;
                    let src_pixel_idx = src_y * full_width as usize + src_x;
                    let dst_pixel_idx = row * region_width + col;

                    // Read pixel from source
                    let src_byte_idx = src_pixel_idx / 2;
                    let src_is_high = (src_pixel_idx & 1) == 0;
                    let pixel = if src_is_high {
                        (full_buffer[src_byte_idx] >> 4) & 0x0F
                    } else {
                        full_buffer[src_byte_idx] & 0x0F
                    };

                    // Write pixel to destination
                    let dst_byte_idx = dst_pixel_idx / 2;
                    let dst_is_high = (dst_pixel_idx & 1) == 0;
                    if dst_is_high {
                        region_buffer[dst_byte_idx] =
                            (region_buffer[dst_byte_idx] & 0x0F) | (pixel << 4);
                    } else {
                        region_buffer[dst_byte_idx] = (region_buffer[dst_byte_idx] & 0xF0) | pixel;
                    }
                }
            }

            region_buffer
        }
        _ => {
            // For non-packed formats, use the simple byte-based approach
            let bytes_per_pixel = match pixel_format {
                PixelFormat::Rgb565 => 2,
                _ => 1,
            };
            extract_region_buffer(
                full_buffer,
                region,
                full_width,
                full_height,
                bytes_per_pixel,
            )
        }
    }
}
