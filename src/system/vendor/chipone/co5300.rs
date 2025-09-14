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
use crate::libs::gfx::two_d::{Point, Size, Rect};
use crate::system::hal::display::{AsyncDisplay, Orientation, PixelFormat, DisplayCapabilities, DisplayResolution, DisplaySize};
use crate::system::kernel::config::resources::{FRAME_BUFFER_SIZE};

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

/// Map unified PixelFormat to Chipone COLMOD register value
fn chipone_colmod_value(fmt: PixelFormat) -> u8 {
    match fmt {
        PixelFormat::Rgb565 => 0x55,
        PixelFormat::Rgb888 => 0x77,
        PixelFormat::Rgb666 => 0x66,
        PixelFormat::Gray8 => 0x11,
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
    pixel_format: PixelFormat,
    /// Active logical resolution/scale (logical against physical panel).
    active_resolution: DisplayResolution,
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
        pixel_format: PixelFormat,
    ) -> Self {
        info!("Creating Co5300 driver");
        let physical = DisplaySize { width: width as u32, height: height as u32 };
        // Default logical to 116x116 with scale=4 if panel dimensions fit; else fall back to 1x
        let (logical, scale) = if (width as u32) >= 116 * 4 && (height as u32) >= 116 * 4 {
            (DisplaySize { width: 116, height: 116 }, 4u32)
        } else {
            (DisplaySize { width: width as u32, height: height as u32 }, 1u32)
        };
        let active_resolution = DisplayResolution { logical, physical, scale };
        Self {
            qspi,
            reset_pin,
            width,
            height,
            x_gap: 6, // Default gap from original code
            y_gap: 0,
            pixel_format,
            active_resolution,
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

    /// Draw region with scale=1 (no scaling). Sends the buffer directly.
    async fn draw_region_scale1(&mut self, buffer: &[u8], region: Rect) {
        let region_x = region.top_left.x as u16;
        let region_y = region.top_left.y as u16;
        let region_width = region.size.width as u16;
        let region_height = region.size.height as u16;

        if let Err(_) = self.set_window(
            region_x,
            region_y,
            region_x + region_width,
            region_y + region_height
        ).await { return; }

        if let Err(_) = self.send_pixels(buffer).await {
            error!("Failed to send pixels for scale=1 draw_region");
            return;
        }
    }

    /// Draw region with generic integer scale (e.g., 2).
    async fn draw_region_scale_generic(&mut self, buffer: &[u8], region: Rect, scale: u16) {
        let region_x = region.top_left.x as u16;
        let region_y = region.top_left.y as u16;
        let region_width = region.size.width as u16;
        let region_height = region.size.height as u16;

        let display_x = region_x * scale;
        let display_y = region_y * scale;
        let display_width = region.size.width * (scale as u32);
        let display_height = region.size.height * (scale as u32);

        const CHUNK_HEIGHT: u16 = 50;
        let scaled_width = (region_width as u32 * scale as u32) as u16;

        for y_chunk_start in (0..display_height as u16).step_by(CHUNK_HEIGHT as usize) {
            let chunk_height = core::cmp::min(CHUNK_HEIGHT, display_height as u16 - y_chunk_start);

            if let Err(_) = self.set_window(
                display_x,
                display_y + y_chunk_start,
                display_x + display_width as u16,
                display_y + y_chunk_start + chunk_height
            ).await { return; }

            let mut chunk_buffer: Vec<u8> = vec![0u8; (scaled_width as usize) * (chunk_height as usize) * 2];

            for row in 0..chunk_height as usize {
                let src_row = (row + y_chunk_start as usize) / scale as usize;
                for col in 0..(scaled_width as usize) {
                    let src_col = col / scale as usize;
                    let src_index = ((src_row * region_width as usize) + src_col) * 2;
                    let dst_index = ((row * scaled_width as usize) + col) * 2;
                    chunk_buffer[dst_index] = buffer[src_index];
                    chunk_buffer[dst_index + 1] = buffer[src_index + 1];
                }
            }

            if let Err(_) = self.send_pixels(&chunk_buffer).await { error!("Failed to send pixels for draw_region chunk (generic)"); return; }
        }
    }

    /// Draw region with optimized scale=4 path.
    async fn draw_region_scale4(&mut self, buffer: &[u8], region: Rect) {
        let region_x = region.top_left.x as u16;
        let region_y = region.top_left.y as u16;
        let region_width = region.size.width as u16;
        let region_height = region.size.height as u16;

        let scale: u16 = 4;
        let display_x = region_x * scale;
        let display_y = region_y * scale;
        let display_width = region.size.width * (scale as u32);
        let display_height = region.size.height * (scale as u32);

        const CHUNK_HEIGHT: u16 = 50;

        let scaled_width = region_width * scale as u16;
        let row_u64s: usize = (scaled_width as usize) / 4; // equals region_width as usize when scale==4
        let mut chunk_buffer: Vec<u64> = vec![0u64; row_u64s * (CHUNK_HEIGHT as usize)];
        let mut scaled_row_buffer: Vec<u64> = vec![0u64; row_u64s];

        for y_chunk_start in (0..display_height as u16).step_by(CHUNK_HEIGHT as usize) {
            let chunk_height = core::cmp::min(CHUNK_HEIGHT, display_height as u16 - y_chunk_start);

            if let Err(_) = self.set_window(
                display_x,
                display_y + y_chunk_start,
                display_x + display_width as u16,
                display_y + y_chunk_start + chunk_height
            ).await { return; }

            unsafe {
                let src_ptr = buffer.as_ptr();
                let mut dst_offset_u64: usize = 0;
                let mut current_src_row = usize::MAX;
                let bytes_per_src_row = region_width as usize * 2; // 2 bytes per pixel (RGB565)

                for row in 0..chunk_height as usize {
                    let src_row = (row + y_chunk_start as usize) / scale as usize;
                    if src_row != current_src_row {
                        current_src_row = src_row;
                        let src_row_ptr = src_ptr.add(src_row * bytes_per_src_row);
                        for src_col in 0..(region_width as usize) {
                            let pixel: u16 = core::ptr::read_unaligned(src_row_ptr.add(src_col * 2) as *const u16);
                            let pixel_u64 = (pixel as u64)
                                | ((pixel as u64) << 16)
                                | ((pixel as u64) << 32)
                                | ((pixel as u64) << 48);
                            scaled_row_buffer[src_col] = pixel_u64;
                        }
                    }
                    let dst_row_ptr = chunk_buffer.as_mut_ptr().add(dst_offset_u64);
                    core::ptr::copy_nonoverlapping(
                        scaled_row_buffer.as_ptr(),
                        dst_row_ptr,
                        row_u64s,
                    );
                    dst_offset_u64 += row_u64s;
                }
            }

            let chunk_bytes_len = (row_u64s * (chunk_height as usize)) * core::mem::size_of::<u64>();
            let chunk_bytes: &[u8] = unsafe {
                core::slice::from_raw_parts(
                    chunk_buffer.as_ptr() as *const u8,
                    chunk_bytes_len,
                )
            };
            if let Err(_) = self.send_pixels(chunk_bytes).await { error!("Failed to send pixels for draw_region chunk (scale4)"); return; }
        }
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
            self.send_command_with_data(commands::COLMOD, &[chipone_colmod_value(self.pixel_format)]).await?;

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

    async fn draw_region(&mut self, buffer: &[u8], region: Rect) {
        let scale = self.active_resolution.scale as u16;
        match scale {
            1 => self.draw_region_scale1(buffer, region).await,
            2 => self.draw_region_scale_generic(buffer, region, 2).await,
            4 => self.draw_region_scale4(buffer, region).await,
            s => self.draw_region_scale_generic(buffer, region, s).await,
        }
    }
    async fn draw(&mut self, buffer: &[u8]) {
        let lw = self.active_resolution.logical.width;
        let lh = self.active_resolution.logical.height;
        let full_region = Rect::new(Point::new(0, 0), Size::new(lw, lh));
        self.draw_region(buffer, full_region).await;
    }

    async fn set_orientation(&mut self, _orientation: Orientation) {
        // Not implemented as requested
    }

    fn get_width(&self) -> u32 {
        self.active_resolution.logical.width
    }

    fn get_height(&self) -> u32 {
        self.active_resolution.logical.height
    }

    fn native_pixel_format(&self) -> PixelFormat {
        PixelFormat::Rgb565
    }

    fn capabilities(&self) -> DisplayCapabilities {
        // Provide common modes: 466x466 (1x), 233x233 (2x), 116x116 (4x)
        // Physical is fixed: panel size.
        const PHYS_W: u32 = 466;
        const PHYS_H: u32 = 466;
        const SUPPORTED: &[DisplayResolution] = &[
            DisplayResolution { logical: DisplaySize { width: 466, height: 466 }, physical: DisplaySize { width: PHYS_W, height: PHYS_H }, scale: 1 },
            DisplayResolution { logical: DisplaySize { width: 233, height: 233 }, physical: DisplaySize { width: PHYS_W, height: PHYS_H }, scale: 2 },
            DisplayResolution { logical: DisplaySize { width: 116, height: 116 }, physical: DisplaySize { width: PHYS_W, height: PHYS_H }, scale: 4 },
        ];
        DisplayCapabilities {
            supported_formats: &[PixelFormat::Rgb565],
            preferred_format: PixelFormat::Rgb565,
            supported_resolutions: SUPPORTED,
            preferred_resolution: SUPPORTED[2], // default to 116x116 @ 4x
        }
    }

    fn set_resolution(&mut self, resolution: DisplayResolution) {
        // Accept only supported scales: 1,2,4. Fallback to nearest.
        let scale = match resolution.scale {
            1 | 2 | 4 => resolution.scale,
            s if s < 2 => 1,
            s if s < 4 => 2,
            _ => 4,
        };
        let physical = DisplaySize { width: self.width as u32, height: self.height as u32 };
        let logical = if scale == 1 { physical } else { DisplaySize { width: physical.width / scale, height: physical.height / scale } };
        self.active_resolution = DisplayResolution { logical, physical, scale };
    }
}