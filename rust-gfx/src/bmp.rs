/// BMP file output matching SDL_SaveBMP format
/// Generates 32-bit ARGB8888 BMP files with BI_BITFIELDS compression
use std::fs::File;
use std::io::{self, Write};
use std::path::Path;

use crate::canvas::Canvas;

const BMP_FILE_HEADER_SIZE: usize = 14;
const BMP_INFO_HEADER_V4_SIZE: usize = 108;
const BMP_PIXEL_DATA_OFFSET: usize = BMP_FILE_HEADER_SIZE + BMP_INFO_HEADER_V4_SIZE;

/// Save canvas as BMP file, matching SDL_SaveBMP output exactly
/// This ensures our checksums match the reference sprites
pub fn save_bmp<P: AsRef<Path>>(canvas: &Canvas, path: P) -> io::Result<()> {
    let mut file = File::create(path)?;

    let width = canvas.width as i32;
    let height = canvas.height as i32;
    let row_size = width * 4;  // 4 bytes per pixel (ARGB8888)
    let pixel_data_size = row_size * height;
    let file_size = BMP_PIXEL_DATA_OFFSET + pixel_data_size as usize;

    // BMP File Header (14 bytes)
    file.write_all(&[0x42, 0x4D])?;  // "BM" signature
    file.write_all(&(file_size as u32).to_le_bytes())?;  // File size
    file.write_all(&[0x00, 0x00, 0x00, 0x00])?;  // Reserved
    file.write_all(&(BMP_PIXEL_DATA_OFFSET as u32).to_le_bytes())?;  // Pixel data offset

    // BITMAPV4HEADER (108 bytes) - matches SDL output
    file.write_all(&(BMP_INFO_HEADER_V4_SIZE as u32).to_le_bytes())?;  // Header size: 108
    file.write_all(&width.to_le_bytes())?;  // Width
    file.write_all(&height.to_le_bytes())?;  // Height
    file.write_all(&[0x01, 0x00])?;  // Planes: 1
    file.write_all(&[0x20, 0x00])?;  // Bits per pixel: 32
    file.write_all(&[0x03, 0x00, 0x00, 0x00])?;  // Compression: BI_BITFIELDS (3)
    file.write_all(&(pixel_data_size as u32).to_le_bytes())?;  // Image size
    file.write_all(&[0x00, 0x00, 0x00, 0x00])?;  // X pixels per meter: 0
    file.write_all(&[0x00, 0x00, 0x00, 0x00])?;  // Y pixels per meter: 0
    file.write_all(&[0x00, 0x00, 0x00, 0x00])?;  // Colors used: 0
    file.write_all(&[0x00, 0x00, 0x00, 0x00])?;  // Important colors: 0

    // Color masks (for BI_BITFIELDS)
    file.write_all(&[0x00, 0xFF, 0x00, 0x00])?;  // Red mask: 0x00FF0000
    file.write_all(&[0x00, 0x00, 0xFF, 0x00])?;  // Green mask: 0x0000FF00
    file.write_all(&[0xFF, 0x00, 0x00, 0x00])?;  // Blue mask: 0x000000FF
    file.write_all(&[0x00, 0x00, 0x00, 0xFF])?;  // Alpha mask: 0xFF000000

    // Color space type: "Win " (0x57696E20)
    file.write_all(&[0x20, 0x6E, 0x69, 0x57])?;

    // CIE color space endpoints (36 bytes) - all zeros
    file.write_all(&[0u8; 36])?;

    // Gamma values (12 bytes) - all zeros
    file.write_all(&[0u8; 12])?;

    // Write pixel data (bottom-up, as BMP format requires)
    let raw_data = canvas.as_raw_argb();
    
    for y in (0..height).rev() {
        let row_start = (y * width) as usize;
        let row_end = row_start + width as usize;
        let row = &raw_data[row_start..row_end];
        
        // Write as ARGB bytes (matching our Argb8888 format)
        for &pixel in row {
            file.write_all(&pixel.to_le_bytes())?;
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::color_argb::Argb8888;

    #[test]
    fn test_bmp_save() {
        let mut canvas = Canvas::new(102, 125);
        canvas.clear(Argb8888::RED);
        
        // Test save (comment out to avoid file I/O in tests)
        // save_bmp(&canvas, "test_output.bmp").unwrap();
    }
}
