#![no_std]
extern crate alloc;

use alloc::boxed::Box;
use alloc::vec;
use alloc::vec::Vec;
use async_trait::async_trait;
use embedded_hal::digital::OutputPin;
use embassy_time::{Duration, Timer};
use esp_hal::spi::master::{Address, Command, DataMode, SpiDmaBus};
use defmt::{info, error};
use embedded_graphics_core::prelude::{Point, Size};
use embedded_graphics_core::primitives::Rectangle;
use crate::system::hal::display::{AsyncDisplay, Orientation};
use crate::system::kernel::config::resources::{FRAME_BUFFER_HEIGHT, FRAME_BUFFER_SIZE, FRAME_BUFFER_WIDTH, FRAME_SCALE_FACTOR};

/// SH8601 Command Set
pub mod commands {
    pub const NOP: u8 = 0x00;
    pub const SWRESET: u8 = 0x01;
    pub const SLPIN: u8 = 0x10;
    pub const SLPOUT: u8 = 0x11;
    pub const INVOFF: u8 = 0x20;
    pub const INVON: u8 = 0x21;
    pub const DISPOFF: u8 = 0x28;
    pub const DISPON: u8 = 0x29;
    pub const CASET: u8 = 0x2A;
    pub const PASET: u8 = 0x2B;
    pub const RAMWR: u8 = 0x2C;
    pub const TEON: u8 = 0x35;
    pub const MADCTL: u8 = 0x36;
    pub const COLMOD: u8 = 0x3A;
    pub const RAMWRC: u8 = 0x3C;
    pub const TESCAN: u8 = 0x44;
    pub const WRDISBV: u8 = 0x51;
    pub const WRCTRLD1: u8 = 0x53;
    pub const C4: u8 = 0xC4;
    pub const C63: u8 = 0x63;
}

/// Supported color modes
#[derive(Debug, Clone, Copy)]
pub enum ColorMode {
    Rgb565,
    Rgb888,
    Rgb666,
    Gray8,
}

impl ColorMode {
    pub const fn bytes_per_pixel(&self) -> usize {
        match self {
            ColorMode::Rgb565 => 2,
            ColorMode::Rgb888 => 3,
            ColorMode::Rgb666 => 3,
            ColorMode::Gray8 => 1,
        }
    }

    pub const fn colmod_value(&self) -> u8 {
        match self {
            ColorMode::Rgb565 => 0x55,
            ColorMode::Rgb888 => 0x77,
            ColorMode::Rgb666 => 0x66,
            ColorMode::Gray8 => 0x11,
        }
    }
}

impl Orientation {
    fn display_mode(&self) -> u8 {
        match self {
            Self::Portrait => 0x40 | 0x08,
            Self::Landscape => 0x20 | 0x08,
            Self::PortraitFlipped => 0x80 | 0x08,
            Self::LandscapeFlipped => 0x40 | 0x80 | 0x20 | 0x08,
        }
    }

    fn is_landscape(&self) -> bool {
        matches!(self, Self::Landscape | Self::LandscapeFlipped)
    }
}

// Precomputed LUT for 4-bit grayscale to RGB565
const GRAY4_LUT: [u16; 16] = {
    let mut lut = [0u16; 16];
    let mut gray = 0;
    while gray < 16 {
        let intensity = (gray * 255 / 15) as u16;
        let r = (intensity >> 3) & 0x1F;
        let g = (intensity >> 2) & 0x3F;
        let b = (intensity >> 3) & 0x1F;
        lut[gray as usize] = (r << 11) | (g << 5) | b;
        gray += 1;
    }
    lut
};

/// QSPI constants
const QSPI_PIXEL_OPCODE: u8 = 0x32;
const QSPI_CONTROL_OPCODE: u8 = 0x02;
const DMA_CHUNK_SIZE: usize = 16380;
const CMD_RAMWR: u32 = 0x2C;
const CMD_RAMWRC: u32 = 0x3C;

/// Driver for SH8601-based 1.8" AMOLED
pub struct Co5300<RST> {
    qspi: SpiDmaBus<'static, esp_hal::Async>,
    reset_pin: RST,
    width: u16,
    height: u16,
    x_gap: u16,
    y_gap: u16,
    color_mode: ColorMode,
}

impl<RST> Co5300<RST>
where
    RST: OutputPin
{
    pub fn new(
        qspi: SpiDmaBus<'static, esp_hal::Async>,
        reset_pin: RST,
        width: u16,
        height: u16,
        color_mode: ColorMode,
    ) -> Self {
        info!("Creating Co5300 driver");
        Self {
            qspi,
            reset_pin,
            width,
            height,
            x_gap: 6, // Default gap from original code
            y_gap: 0,
            color_mode,
        }
    }

    /// Send a command via QSPI
    async fn send_command(&mut self, cmd: u8) -> Result<(), esp_hal::spi::Error> {
        let address_value = (cmd as u32) << 8;

        self.qspi.half_duplex_write(
            DataMode::Single,
            Command::_8Bit(QSPI_CONTROL_OPCODE as u16, DataMode::Single),
            Address::_24Bit(address_value, DataMode::Single),
            0,
            &[],
        )?;
        Ok(())
    }

    /// Send a command with data via QSPI
    async fn send_command_with_data(&mut self, cmd: u8, data: &[u8]) -> Result<(), esp_hal::spi::Error> {
        let address_value = (cmd as u32) << 8;

        self.qspi.half_duplex_write(
            DataMode::Single,
            Command::_8Bit(QSPI_CONTROL_OPCODE as u16, DataMode::Single),
            Address::_24Bit(address_value, DataMode::Single),
            0,
            data,
        )?;
        Ok(())
    }

    /// Send pixel data via QSPI
    async fn send_pixels(&mut self, pixels: &[u8]) -> Result<(), esp_hal::spi::Error> {
        let ramwr_addr_val = (CMD_RAMWR as u32) << 8;
        let ramwrc_addr_val = (CMD_RAMWRC as u32) << 8;

        let mut chunks = pixels.chunks(DMA_CHUNK_SIZE).enumerate();

        while let Some((index, chunk)) = chunks.next() {
            let addr_val = if index == 0 { ramwr_addr_val } else { ramwrc_addr_val };

            self.qspi.half_duplex_write(
                DataMode::Quad,
                Command::_8Bit(QSPI_PIXEL_OPCODE as u16, DataMode::Single),
                Address::_24Bit(addr_val, DataMode::Single),
                0,
                chunk,
            )?;
        }
        Ok(())
    }

    /// Perform hardware reset
    async fn hard_reset(&mut self) -> Result<(), ()> {
        info!("Performing hard reset");

        self.reset_pin.set_low().map_err(|_| ())?;
        Timer::after(Duration::from_millis(10)).await;
        self.reset_pin.set_high().map_err(|_| ())?;
        Timer::after(Duration::from_millis(150)).await;

        Ok(())
    }

    /// Set display region for drawing
    async fn set_window(&mut self, x_start: u16, y_start: u16, x_end: u16, y_end: u16) -> Result<(), esp_hal::spi::Error> {
        let x_start = x_start + self.x_gap;
        let x_end = x_end + self.x_gap;
        let y_start = y_start + self.y_gap;
        let y_end = y_end + self.y_gap;

        self.send_command_with_data(
            commands::CASET,
            &[
                (x_start >> 8) as u8,
                (x_start & 0xFF) as u8,
                ((x_end - 1) >> 8) as u8,
                ((x_end - 1) & 0xFF) as u8,
            ],
        ).await?;

        self.send_command_with_data(
            commands::PASET,
            &[
                (y_start >> 8) as u8,
                (y_start & 0xFF) as u8,
                ((y_end - 1) >> 8) as u8,
                ((y_end - 1) & 0xFF) as u8,
            ],
        ).await?;

        Ok(())
    }
}

#[async_trait(?Send)]
impl<RST> AsyncDisplay for Co5300<RST>
where
    RST: OutputPin + Send,
{
    async fn init(&mut self) {
        info!("Initializing Co5300 display");

        if let Err(_) = self.hard_reset().await {
            error!("Hard reset failed");
            return;
        }

        // Initialize display with sequence from original code
        let init_result = async {
            self.send_command(commands::SLPOUT).await?;
            Timer::after(Duration::from_millis(80)).await;

            self.send_command_with_data(commands::C4, &[0x80]).await?;
            self.send_command_with_data(commands::WRCTRLD1, &[0x20]).await?;
            Timer::after(Duration::from_millis(1)).await;

            self.send_command_with_data(commands::C63, &[0xFF]).await?;
            Timer::after(Duration::from_millis(1)).await;

            self.send_command_with_data(commands::WRDISBV, &[0x00]).await?;
            Timer::after(Duration::from_millis(1)).await;

            self.send_command(commands::DISPON).await?;
            Timer::after(Duration::from_millis(10)).await;

            self.send_command_with_data(commands::WRDISBV, &[0xFF]).await?;

            // Vendor-specific initialization
            self.send_command_with_data(commands::TESCAN, &[0x00, 0xC8]).await?;
            self.send_command_with_data(commands::TEON, &[0x00]).await?;
            self.send_command_with_data(commands::WRCTRLD1, &[0x20]).await?;
            Timer::after(Duration::from_millis(25)).await;

            // Set pixel format and MADCTL
            self.send_command_with_data(commands::MADCTL, &[0x00]).await?;
            self.send_command_with_data(commands::COLMOD, &[self.color_mode.colmod_value()]).await?;

            self.send_command(commands::DISPON).await?;

            Ok::<(), esp_hal::spi::Error>(())
        }.await;

        match init_result {
            Ok(_) => info!("Display initialization complete"),
            Err(_) => error!("Display initialization failed"),
        }
    }

    async fn paint_screen(&mut self, color: u16) {
        info!("Painting screen with color");

        const CHUNK_HEIGHT: u16 = 10;
        let chunk_pixels = self.width * CHUNK_HEIGHT;
        let chunk_size = (chunk_pixels * 2) as usize; // RGB565 = 2 bytes per pixel

        let mut buffer = vec![0u8; chunk_size];

        // Fill buffer with color
        for i in 0..(chunk_pixels as usize) {
            let idx = i * 2;
            buffer[idx] = (color >> 8) as u8;
            buffer[idx + 1] = color as u8;
        }

        for y in (0..self.height).step_by(CHUNK_HEIGHT as usize) {
            let height = if y + CHUNK_HEIGHT <= self.height {
                CHUNK_HEIGHT
            } else {
                self.height - y
            };

            if let Err(_) = self.set_window(0, y, self.width, y + height).await {
                error!("Failed to set window for paint_screen");
                return;
            }

            let pixels_to_send = (self.width * height * 2) as usize;
            if let Err(_) = self.send_pixels(&buffer[0..pixels_to_send]).await {
                error!("Failed to send pixels for paint_screen");
                return;
            }
        }
    }

    async fn set_brightness(&mut self, value: u8) {
        info!("Setting brightness to {}", value);

        if let Err(_) = self.send_command_with_data(commands::WRDISBV, &[value]).await {
            error!("Failed to set brightness");
        }
    }

    async fn draw_gray4(&mut self, buffer: &[u8], scale: u32) {
        info!("Drawing gray4 buffer, scale: {}", scale);

        if scale != FRAME_SCALE_FACTOR {
            error!("Invalid scale: {} (expected {})", scale, FRAME_SCALE_FACTOR);
            return;
        }

        if buffer.len() < FRAME_BUFFER_SIZE {
            error!("Buffer too small: {} < {}", buffer.len(), FRAME_BUFFER_SIZE);
            return;
        }

        const CHUNK_HEIGHT: u16 = 25;
        let chunk_pixels = self.width * CHUNK_HEIGHT;
        let chunk_size = (chunk_pixels * 2) as usize; // RGB565 output

        let mut rgb565_buffer = vec![0u8; chunk_size];

        for y in (0..self.height).step_by(CHUNK_HEIGHT as usize) {
            let actual_height = if y + CHUNK_HEIGHT <= self.height {
                CHUNK_HEIGHT
            } else {
                self.height - y
            };

            // Set window for this chunk
            if let Err(_) = self.set_window(0, y, self.width, y + actual_height).await {
                error!("Failed to set window for gray4 chunk at y={}", y);
                return;
            }

            // Convert gray4 to RGB565 for this chunk
            for row in 0..actual_height {
                let display_y = y + row;
                let frame_y = display_y / scale as u16;

                for col in 0..self.width {
                    let frame_x = col / scale as u16;

                    // Calculate source position in 4-bit buffer
                    // Canvas Gray4 layout: pixel_index = x + y * width
                    let pixel_idx = (frame_x as usize) + (frame_y as usize * FRAME_BUFFER_WIDTH as usize);
                    let byte_idx = pixel_idx / 2;
                    let is_high_nibble = pixel_idx % 2 == 0;

                    // Extract 4-bit value
                    let gray4_val = if byte_idx < buffer.len() {
                        if is_high_nibble {
                            (buffer[byte_idx] >> 4) & 0x0F
                        } else {
                            buffer[byte_idx] & 0x0F
                        }
                    } else {
                        0
                    };

                    // Convert to RGB565 using LUT
                    let rgb565 = GRAY4_LUT[gray4_val as usize];

                    // Store in output buffer
                    let out_idx = ((row * self.width + col) * 2) as usize;
                    if out_idx + 1 < rgb565_buffer.len() {
                        rgb565_buffer[out_idx] = (rgb565 >> 8) as u8;
                        rgb565_buffer[out_idx + 1] = rgb565 as u8;
                    }
                }
            }

            // Send this chunk
            let pixels_to_send = (self.width * actual_height * 2) as usize;
            if let Err(_) = self.send_pixels(&rgb565_buffer[0..pixels_to_send]).await {
                error!("Failed to send pixels for gray4 chunk");
                return;
            }
        }
    }

    async fn draw_gray4_region(&mut self, buffer: &[u8], region: Rectangle, scale: u32) {
        info!("Drawing gray4 region, scale: {}", scale);

        if scale != FRAME_SCALE_FACTOR {
            error!("Invalid scale: {} (expected {})", scale, FRAME_SCALE_FACTOR);
            return;
        }

        if buffer.len() < FRAME_BUFFER_SIZE {
            error!("Buffer too small: {} < {}", buffer.len(), FRAME_BUFFER_SIZE);
            return;
        }

        // Scale the region coordinates to display space
        let scaled_region = Rectangle::new(
            Point::new(region.top_left.x * scale as i32, region.top_left.y * scale as i32),
            Size::new(region.size.width * scale, region.size.height * scale)
        );

        // Validate and clip region bounds to display
        let x_start = scaled_region.top_left.x.max(0) as u16;
        let y_start = scaled_region.top_left.y.max(0) as u16;
        let x_end = (scaled_region.top_left.x + scaled_region.size.width as i32).min(self.width as i32) as u16;
        let y_end = (scaled_region.top_left.y + scaled_region.size.height as i32).min(self.height as i32) as u16;

        if x_start >= x_end || y_start >= y_end {
            error!("Invalid region bounds after scaling");
            return;
        }

        if let Err(_) = self.set_window(x_start, y_start, x_end, y_end).await {
            error!("Failed to set window for gray4 region");
            return;
        }

        let region_width = x_end - x_start;
        let region_height = y_end - y_start;

        const CHUNK_HEIGHT: u16 = 25;
        let chunk_pixels = region_width * CHUNK_HEIGHT;
        let chunk_size = (chunk_pixels * 2) as usize;
        let mut rgb565_buffer = vec![0u8; chunk_size];

        for y in (0..region_height).step_by(CHUNK_HEIGHT as usize) {
            let actual_height = if y + CHUNK_HEIGHT <= region_height {
                CHUNK_HEIGHT
            } else {
                region_height - y
            };

            // Convert gray4 to RGB565 for this chunk
            for row in 0..actual_height {
                let display_y = y_start + y + row;
                let frame_y = display_y / scale as u16;

                for col in 0..region_width {
                    let display_x = x_start + col;
                    let frame_x = display_x / scale as u16;

                    // Calculate source position in 4-bit buffer
                    let pixel_idx = (frame_x as usize) + (frame_y as usize * FRAME_BUFFER_WIDTH as usize);
                    let byte_idx = pixel_idx / 2;
                    let is_high_nibble = pixel_idx % 2 == 0;

                    // Extract 4-bit value
                    let gray4_val = if byte_idx < buffer.len() {
                        if is_high_nibble {
                            (buffer[byte_idx] >> 4) & 0x0F
                        } else {
                            buffer[byte_idx] & 0x0F
                        }
                    } else {
                        0
                    };

                    // Convert to RGB565 using LUT
                    let rgb565 = GRAY4_LUT[gray4_val as usize];

                    // Store in output buffer
                    let out_idx = ((row * region_width + col) * 2) as usize;
                    if out_idx + 1 < rgb565_buffer.len() {
                        rgb565_buffer[out_idx] = (rgb565 >> 8) as u8;
                        rgb565_buffer[out_idx + 1] = rgb565 as u8;
                    }
                }
            }

            // Send this chunk
            let pixels_to_send = (region_width * actual_height * 2) as usize;
            if let Err(_) = self.send_pixels(&rgb565_buffer[0..pixels_to_send]).await {
                error!("Failed to send pixels for gray4 region chunk");
                return;
            }
        }
    }

    async fn draw(&mut self, buffer: &[u8], scale: u32) {
        info!("Drawing RGB565 buffer, scale: {}", scale);

        if ![1, 2, 4].contains(&scale) {
            error!("Invalid scale: {}", scale);
            return;
        }

        let expected_size = (self.width * self.height * 2) as usize;
        if buffer.len() < expected_size {
            error!("Buffer too small: {} < {}", buffer.len(), expected_size);
            return;
        }

        let scaled_width = self.width * scale as u16;
        let scaled_height = self.height * scale as u16;

        if let Err(_) = self.set_window(0, 0, scaled_width, scaled_height).await {
            error!("Failed to set window for RGB565 draw");
            return;
        }

        let mut scaled_row = vec![0u8; (self.width * scale as u16 * 2) as usize];

        for y in 0..self.height {
            let row_offset = (y * self.width * 2) as usize;

            // Scale the row horizontally
            for x in 0..self.width {
                let src_idx = row_offset + (x * 2) as usize;
                let rgb565_bytes = &buffer[src_idx..src_idx + 2];

                let base_idx = (x * scale as u16 * 2) as usize;
                for s in 0..scale as usize {
                    let dst_idx = base_idx + s * 2;
                    scaled_row[dst_idx] = rgb565_bytes[0];
                    scaled_row[dst_idx + 1] = rgb565_bytes[1];
                }
            }

            // Send scaled rows vertically
            for _ in 0..scale {
                if let Err(_) = self.send_pixels(&scaled_row).await {
                    error!("Failed to send pixels for RGB565 row");
                    return;
                }
            }
        }
    }

    async fn clear(&mut self, color: u16) {
        info!("Clearing screen");
        self.paint_screen(color).await;
    }

    async fn set_orientation(&mut self, _orientation: Orientation) {
        // Not implemented as requested
    }

    fn get_width(&self) -> u32 {
        self.width as u32
    }

    fn get_height(&self) -> u32 {
        self.height as u32
    }
}