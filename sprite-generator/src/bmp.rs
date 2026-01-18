use crate::canvas::Canvas;
use std::fs::File;
use std::io::{self, Write};
use std::path::Path;

const BMP_FILE_HEADER_SIZE: usize = 14;
const BMP_INFO_HEADER_V4_SIZE: usize = 108;
const BMP_PIXEL_DATA_OFFSET: usize = BMP_FILE_HEADER_SIZE + BMP_INFO_HEADER_V4_SIZE;

pub fn save_bmp<P: AsRef<Path>>(canvas: &Canvas, path: P) -> io::Result<()> {
    let mut file = File::create(path)?;

    let width = canvas.width as i32;
    let height = canvas.height as i32;
    let row_size = width * 4;
    let pixel_data_size = row_size * height;
    let file_size = BMP_PIXEL_DATA_OFFSET + pixel_data_size as usize;

    file.write_all(&[0x42, 0x4D])?;
    file.write_all(&(file_size as u32).to_le_bytes())?;
    file.write_all(&[0x00, 0x00, 0x00, 0x00])?;
    file.write_all(&(BMP_PIXEL_DATA_OFFSET as u32).to_le_bytes())?;

    file.write_all(&(BMP_INFO_HEADER_V4_SIZE as u32).to_le_bytes())?;
    file.write_all(&width.to_le_bytes())?;
    file.write_all(&height.to_le_bytes())?;
    file.write_all(&[0x01, 0x00])?;
    file.write_all(&[0x20, 0x00])?;
    file.write_all(&[0x03, 0x00, 0x00, 0x00])?;
    file.write_all(&(pixel_data_size as u32).to_le_bytes())?;
    file.write_all(&[0x00, 0x00, 0x00, 0x00])?;
    file.write_all(&[0x00, 0x00, 0x00, 0x00])?;
    file.write_all(&[0x00, 0x00, 0x00, 0x00])?;
    file.write_all(&[0x00, 0x00, 0x00, 0x00])?;

    file.write_all(&[0x00, 0x00, 0xFF, 0x00])?;
    file.write_all(&[0x00, 0xFF, 0x00, 0x00])?;
    file.write_all(&[0xFF, 0x00, 0x00, 0x00])?;
    file.write_all(&[0x00, 0x00, 0x00, 0xFF])?;

    file.write_all(&[0x20, 0x6E, 0x69, 0x57])?;
    file.write_all(&[0u8; 36])?;
    file.write_all(&[0u8; 12])?;

    for y in (0..height).rev() {
        let row_start = (y * width) as usize;
        let row_end = row_start + width as usize;
        let row = &canvas.buffer()[row_start..row_end];

        for &pixel in row {
            file.write_all(&[pixel.b(), pixel.g(), pixel.r(), pixel.a()])?;
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_gfx::color::Rgba8888;

    #[test]
    fn test_bmp_save() {
        let mut canvas = Canvas::new(4, 4);
        canvas.clear(Rgba8888::WHITE);
        let path = Path::new("test_output.bmp");
        save_bmp(&canvas, path).unwrap();
        std::fs::remove_file(path).unwrap();
    }
}
