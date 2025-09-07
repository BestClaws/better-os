#![no_std]
extern crate alloc;

use alloc::boxed::Box;
use alloc::vec;
use alloc::vec::Vec;
use async_trait::async_trait;
use embedded_hal::digital::OutputPin;
use embassy_time::{Duration, Instant, Timer};
use esp_hal::spi::master::{Address, Command, DataMode, SpiDmaBus};
use defmt::{info, error, debug};
use embedded_graphics_core::prelude::{Point, Size};
use embedded_graphics_core::primitives::Rectangle;
use crate::system::hal::display::{AsyncDisplay, Orientation, DisplayPixelFormat};
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

        debug!(
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
        let frame_start = Instant::now();

        // Extract region parameters
        let region_x = region.top_left.x as u16;
        let region_y = region.top_left.y as u16;
        let region_width = region.size.width as u16;
        let region_height = region.size.height as u16;

        // Calculate display coordinates (scaled)
        let display_x = region_x * scale as u16;
        let display_y = region_y * scale as u16;
        let display_width = region.size.width * scale;
        let display_height = region.size.height * scale;

        const CHUNK_HEIGHT: u16 = 50;

        // Pre-allocate buffers
        let scaled_width = region_width * scale as u16;
        // For scale=4 fast path, each source pixel expands to 4 pixels (8 bytes) -> 1 u64.
        // Use u64-aligned buffers to enable wide copies without misalignment UB.
        let row_u64s: usize = (scaled_width as usize) / 4; // equals region_width as usize when scale==4
        let mut chunk_buffer: Vec<u64> = vec![0u64; row_u64s * (CHUNK_HEIGHT as usize)];
        let mut scaled_row_buffer: Vec<u64> = vec![0u64; row_u64s];

        let mut total_scaling = 0u64;
        let mut total_transfer = 0u64;

        // Process in chunks to avoid huge memory allocation
        for y_chunk_start in (0..display_height as u16).step_by(CHUNK_HEIGHT as usize) {
            let chunk_height = core::cmp::min(CHUNK_HEIGHT, display_height as u16 - y_chunk_start);

            if let Err(_) = self.set_window(
                display_x,
                display_y + y_chunk_start,
                display_x + display_width as u16,
                display_y + y_chunk_start + chunk_height
            ).await {
                return;
            }

            let scale_start = Instant::now();

            unsafe {
                let src_ptr = buffer.as_ptr();
                let mut dst_offset_u64: usize = 0;

                // Optimized scaling for scale=4
                let mut current_src_row = usize::MAX;
                let bytes_per_src_row = region_width as usize * 2; // 2 bytes per pixel (RGB565)

                for row in 0..chunk_height as usize {
                    let src_row = (row + y_chunk_start as usize) / scale as usize;

                    // Regenerate scaled row only when source row advances
                    if src_row != current_src_row {
                        current_src_row = src_row;
                        let src_row_ptr = src_ptr.add(src_row * bytes_per_src_row);

                        // ULTRA-fast row scaling optimized for scale=4
                        for src_col in 0..(region_width as usize) {
                            let pixel: u16 = core::ptr::read_unaligned(src_row_ptr.add(src_col * 2) as *const u16);
                            // Pack 4 copies of the RGB565 pixel into one u64
                            let pixel_u64 = (pixel as u64)
                                | ((pixel as u64) << 16)
                                | ((pixel as u64) << 32)
                                | ((pixel as u64) << 48);
                            // Each entry represents 4 horizontally replicated pixels
                            scaled_row_buffer[src_col] = pixel_u64;
                        }
                    }

                    // Copy one scaled row into chunk buffer using wide copy
                    let dst_row_ptr = chunk_buffer.as_mut_ptr().add(dst_offset_u64);
                    core::ptr::copy_nonoverlapping(
                        scaled_row_buffer.as_ptr(),
                        dst_row_ptr,
                        row_u64s,
                    );
                    dst_offset_u64 += row_u64s;
                }
            }

            total_scaling += scale_start.elapsed().as_micros();
            let transfer_start = Instant::now();

            let chunk_bytes_len = (row_u64s * (chunk_height as usize)) * core::mem::size_of::<u64>();
            let chunk_bytes: &[u8] = unsafe {
                core::slice::from_raw_parts(
                    chunk_buffer.as_ptr() as *const u8,
                    chunk_bytes_len,
                )
            };
            if let Err(_) = self.send_pixels(chunk_bytes).await {
                error!("Failed to send pixels for draw_region chunk");
                return;
            }

            total_transfer += transfer_start.elapsed().as_micros();
        }

        let frame_time = frame_start.elapsed().as_micros();
        info!(
        "draw_region: region: {:?}, scaling {} ms, transfer {} ms, total {} ms",
            region,
        total_scaling as f64 / 1000.0,
        total_transfer as f64 / 1000.0,
        frame_time as f64 / 1000.0
    );
    }
    async fn draw(&mut self, buffer: &[u8], scale: u32) {
        let full_region = Rectangle::new(Point::new(0, 0), Size::new(FRAME_BUFFER_WIDTH, FRAME_BUFFER_HEIGHT));
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

    fn native_pixel_format(&self) -> DisplayPixelFormat {
        DisplayPixelFormat::Rgb565
    }
}