use bmp::{Image, Pixel};

pub fn save_luma4_as_bmp(buffer: &[u8], width: u16, height: u16, filename: &str) {
    let mut img = Image::new(width as u32, height as u32);

    for y in 0..height {
        for x in 0..width {
            let pixel_idx = (y as usize) * (width as usize) + (x as usize);
            let byte_idx = pixel_idx >> 1;
            let is_high_nibble = (pixel_idx & 1) == 0;

            let luma4 = if is_high_nibble {
                (buffer[byte_idx] >> 4) & 0x0F
            } else {
                buffer[byte_idx] & 0x0F
            };

            let luma8 = (luma4 << 4) | luma4;

            img.set_pixel(x as u32, y as u32, Pixel::new(luma8, luma8, luma8));
        }
    }

    img.save(filename).expect("Failed to save BMP");
}

pub fn save_rgb565_as_bmp(buffer: &[u8], width: u16, height: u16, filename: &str) {
    let mut img = Image::new(width as u32, height as u32);

    for y in 0..height {
        for x in 0..width {
            let pixel_offset = (y as usize) * (width as usize) + (x as usize);
            let byte_offset = pixel_offset * 2;

            let rgb565 = u16::from_le_bytes([buffer[byte_offset], buffer[byte_offset + 1]]);

            let r5 = ((rgb565 >> 11) & 0x1F) as u8;
            let g6 = ((rgb565 >> 5) & 0x3F) as u8;
            let b5 = (rgb565 & 0x1F) as u8;

            let r = (r5 << 3) | (r5 >> 2);
            let g = (g6 << 2) | (g6 >> 4);
            let b = (b5 << 3) | (b5 >> 2);

            img.set_pixel(x as u32, y as u32, Pixel::new(r, g, b));
        }
    }

    img.save(filename).expect("Failed to save BMP");
}
