use alloc::vec;
use alloc::vec::Vec as AllocVec;
use crate::libs::gfx::two_d::Rect;

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
        let src_start = ((region_y as usize + row) * full_bytes_per_row) + (region_x as usize * bytes_per_pixel);
        let dst_start = row * region_bytes_per_row;
        region_buffer[dst_start..dst_start + region_bytes_per_row]
            .copy_from_slice(&full_buffer[src_start..src_start + region_bytes_per_row]);
    }
    region_buffer
}


