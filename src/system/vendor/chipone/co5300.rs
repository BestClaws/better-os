#![no_std]
extern crate alloc;

use alloc::boxed::Box;
use alloc::vec;
use alloc::vec::Vec;
use async_trait::async_trait;
use embedded_hal::digital::OutputPin;
use embassy_time::{Duration, Instant, Timer};
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
    async fn paint_screen(&mut self, _color: u8) {
        const SCALE: u16 = 4;
        const REG_X: u16 = 25;
        const REG_Y: u16 = 25;
        const REG_W: u16 = 50;
        const REG_H: u16 = 50;
        const CHUNK_HEIGHT: u16 = 50;
        const SQUARE_SIZE: u16 = 8; // size of squares in region

        // 1. Create buffer for region
        let mut region_buffer = vec![0u8; (REG_W * REG_H * 2) as usize];
        for row in 0..REG_H {
            for col in 0..REG_W {
                let pixel_index = (row * REG_W + col) as usize;
                let pixel_color: u16 = if ((row / SQUARE_SIZE + col / SQUARE_SIZE) % 2) == 0 {
                    0xF800 // Red
                } else {
                    0x001F // Blue
                };
                region_buffer[2 * pixel_index] = (pixel_color >> 8) as u8;
                region_buffer[2 * pixel_index + 1] = pixel_color as u8;
            }
        }

        let scaled_width = REG_W * SCALE;
        let scaled_height = REG_H * SCALE;
        let display_start_x = REG_X * SCALE;
        let display_start_y = REG_Y * SCALE;

        let mut total_compute = 0u64;
        let mut total_transfer = 0u64;

        // 2. Scale and send in chunks
        for y_chunk_start in (0..scaled_height).step_by(CHUNK_HEIGHT as usize) {
            let chunk_height = if y_chunk_start + CHUNK_HEIGHT <= scaled_height {
                CHUNK_HEIGHT
            } else {
                scaled_height - y_chunk_start
            };

            if let Err(_) = self.set_window(
                display_start_x,
                display_start_y + y_chunk_start,
                display_start_x + scaled_width,
                display_start_y + y_chunk_start + chunk_height
            ).await {
                return;
            }

            let mut chunk_buffer = vec![0u8; (scaled_width * chunk_height * 2) as usize];

            let compute_start = Instant::now();
            for row in 0..chunk_height {
                let src_y = (row + y_chunk_start) / SCALE;
                for col in 0..scaled_width {
                    let src_x = col / SCALE;

                    let src_index = (src_y * REG_W + src_x) as usize * 2;
                    let dst_index = (row as usize * scaled_width as usize + col as usize) * 2;

                    chunk_buffer[dst_index] = region_buffer[src_index];
                    chunk_buffer[dst_index + 1] = region_buffer[src_index + 1];
                }
            }
            total_compute += compute_start.elapsed().as_micros();

            let transfer_start = Instant::now();
            if let Err(_) = self.send_pixels(&chunk_buffer).await {
                error!("Failed to send pixels for paint_screen chunk");
                return;
            }
            total_transfer += transfer_start.elapsed().as_micros();
        }

        info!(
        "compute time: {} ms, transfer time: {} ms",
        total_compute as f64 / 1000.0,
        total_transfer as f64 / 1000.0
    );
    }






        async fn set_brightness(&mut self, value: u8) {
        info!("Setting brightness to {}", value);

        if let Err(_) = self.send_command_with_data(commands::WRDISBV, &[value]).await {
            error!("Failed to set brightness");
        }
    }



    async fn draw_region(&mut self, buffer: &[u8], region: Rectangle, scale: u32) {
        let now = Instant::now();
        info!("Drawing region: {:?} with scale: {}", region, scale);

        // Validate region bounds
        if region.is_zero_sized() {
            info!("Zero-sized region, skipping draw");
            return;
        }

        // Calculate the display region considering scale factor
        let display_x = region.top_left.x as u16 * scale as u16;
        let display_y = region.top_left.y as u16 * scale as u16;
        let display_width = region.size.width * scale;
        let display_height = region.size.height * scale;

        // Ensure we don't exceed display bounds
        let display_end_x = (display_x + display_width as u16).min(self.width);
        let display_end_y = (display_y + display_height as u16).min(self.height);

        if display_x >= self.width || display_y >= self.height {
            error!("Region out of bounds");
            return;
        }

        // Set the display window for the scaled region
        if let Err(_) = self.set_window(
            display_x,
            display_y,
            display_end_x,
            display_end_y
        ).await {
            error!("Failed to set window for draw_region");
            return;
        }

        // Calculate source dimensions - RGB565 is 2 bytes per pixel
        let src_width = region.size.width as usize;
        let src_height = region.size.height as usize;
        let expected_buffer_size = src_width * src_height * 2; // RGB565

        if buffer.len() < expected_buffer_size {
            error!("Buffer too small: expected {}, got {}", expected_buffer_size, buffer.len());
            return;
        }

        // For scale=1, we can DMA directly from source buffer
        if scale == 1 {
            let pixels_to_send = (display_width * display_height * 2) as usize;
            if let Err(_) = self.send_pixels(&buffer[0..pixels_to_send]).await {
                error!("Failed to send pixels for draw_region (scale=1)");
            }
            return;
        }

        // For scaling, process in chunks optimized for DMA
        let scaled_width = (display_end_x - display_x) as usize;
        let scaled_height = (display_end_y - display_y) as usize;

        info!("Scaled dimensions: {}x{}, Source: {}x{}", scaled_width, scaled_height, src_width, src_height);

        // Use larger chunks for DMA efficiency, but respect DMA_CHUNK_SIZE limit
        let chunk_height = (DMA_CHUNK_SIZE / (scaled_width * 2)).min(64).max(1);
        let chunk_buffer_size = scaled_width * chunk_height * 2;
        let mut scaled_buffer = vec![0u8; chunk_buffer_size];

        let mut pt = 0;

        for chunk_start_y in (0..scaled_height).step_by(chunk_height) {
            let chunk_end_y = (chunk_start_y + chunk_height).min(scaled_height);
            let chunk_actual_height = chunk_end_y - chunk_start_y;

            info!("Processing chunk: Y={}-{} (height={})", chunk_start_y, chunk_end_y, chunk_actual_height);

            // Need to set window for each chunk since we're sending data sequentially
            let chunk_display_y = display_y + chunk_start_y as u16;
            let chunk_display_end_y = display_y + chunk_end_y as u16;

            if let Err(_) = self.set_window(
                display_x,
                chunk_display_y,
                display_end_x,
                chunk_display_end_y
            ).await {
                error!("Failed to set window for chunk");
                continue;
            }

            // Fill the chunk buffer with scaled pixels
            for (out_row, scaled_y) in (chunk_start_y..chunk_end_y).enumerate() {
                let src_y = scaled_y / scale as usize;
                if src_y >= src_height {
                    error!("Source Y out of bounds: {} >= {}", src_y, src_height);
                    break;
                }

                let src_row_start = src_y * src_width * 2; // RGB565 row start
                let out_row_start = out_row * scaled_width * 2;

                // Scale horizontally with pixel replication
                for scaled_x in 0..scaled_width {
                    let src_x = scaled_x / scale as usize;
                    if src_x >= src_width {
                        break;
                    }

                    let src_pixel_idx = src_row_start + src_x * 2;
                    let out_pixel_idx = out_row_start + scaled_x * 2;

                    // Direct RGB565 copy - no conversion needed
                    if out_pixel_idx + 1 < scaled_buffer.len() && src_pixel_idx + 1 < buffer.len() {
                        scaled_buffer[out_pixel_idx] = buffer[src_pixel_idx];
                        scaled_buffer[out_pixel_idx + 1] = buffer[src_pixel_idx + 1];
                    }
                }
            }

            // Send the chunk via DMA
            let bytes_to_send = chunk_actual_height * scaled_width * 2;
            let now = Instant::now();
            if let Err(_) = self.send_pixels(&scaled_buffer[0..bytes_to_send]).await {
                error!("Failed to send pixels for draw_region chunk");
                return;
            }
            let e = now.elapsed().as_micros();
            info!("elaped chunk t: {}", e);
            pt += e;
        }

        info!("Region drawing completed successfully. time: {}. pt: {}", now.elapsed().as_millis(), pt as f32 / 1000.0);
    }

    async fn draw(&mut self, buffer: &[u8], scale: u32) {
        info!("Drawing full frame buffer with scale: {}", scale);

        // For full screen draws at scale=1, we can optimize further
        if scale == 1 {
            // Set window to full display
            if let Err(_) = self.set_window(0, 0, self.width, self.height).await {
                error!("Failed to set full window");
                return;
            }

            // Direct DMA transfer of the entire buffer
            let expected_size = (FRAME_BUFFER_WIDTH * FRAME_BUFFER_HEIGHT * 2) as usize;
            if buffer.len() >= expected_size {
                if let Err(_) = self.send_pixels(&buffer[0..expected_size]).await {
                    error!("Failed to send full frame buffer");
                }
            } else {
                error!("Buffer too small for full frame: expected {}, got {}", expected_size, buffer.len());
            }
            return;
        }

        // For scaled draws, delegate to draw_region
        let full_region = Rectangle::new(
            Point::new(0, 0),
            Size::new(FRAME_BUFFER_WIDTH, FRAME_BUFFER_HEIGHT)
        );




        self.draw_region(buffer, full_region, scale).await;
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