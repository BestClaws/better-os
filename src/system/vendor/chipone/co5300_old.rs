#![no_std]
extern crate alloc;

use crate::system::hal::display::{
    AsyncDisplay, DisplayCapabilities, DisplayResolution, DisplaySize, Orientation, PixelFormat,
};
use crate::system::kernel::config::resources::{DISPLAY_HEIGHT, DISPLAY_WIDTH};
use crate::util::math::primitives::{Point, Rect, Size};
use alloc::boxed::Box;
use alloc::vec;
use alloc::vec::Vec;
use async_trait::async_trait;
use defmt::{debug, error};
use embassy_time::{Duration, Instant, Timer};
use embedded_hal::digital::OutputPin;
use esp_hal::spi::master::{Address, Command, DataMode, SpiDmaBus};

// ============================================================================
// Constants and Configuration
// ============================================================================

/// Hardware panel offset (from reference implementation)
const HARDWARE_X_OFFSET: u16 = 0x16; // 22 pixels
const HARDWARE_Y_OFFSET: u16 = 0x00; // 0 pixels

/// QSPI communication opcodes
const QSPI_PIXEL_OPCODE: u8 = 0x32;
const QSPI_CONTROL_OPCODE: u8 = 0x02;

/// DMA transfer chunk size for optimal performance
const DMA_CHUNK_SIZE: usize = 16380;

/// Scaling chunk height for memory-efficient scaling operations
const SCALING_CHUNK_HEIGHT: u16 = 50;

/// SH8601 display controller commands
pub mod commands {
    pub const NOP: u8 = 0x00;
    pub const SWRESET: u8 = 0x01;
    pub const SLPIN: u8 = 0x10;
    pub const SLPOUT: u8 = 0x11;
    pub const INVOFF: u8 = 0x20;
    pub const INVON: u8 = 0x21;
    pub const DISPOFF: u8 = 0x28;
    pub const DISPON: u8 = 0x29;
    pub const CASET: u8 = 0x2A; // Column address set
    pub const PASET: u8 = 0x2B; // Page address set
    pub const RAMWR: u8 = 0x2C; // Memory write
    pub const TEON: u8 = 0x35;
    pub const MADCTL: u8 = 0x36; // Memory access control
    pub const COLMOD: u8 = 0x3A; // Pixel format set
    pub const RAMWRC: u8 = 0x3C; // Memory write continue
    pub const TESCAN: u8 = 0x44;
    pub const WRDISBV: u8 = 0x51; // Write display brightness
    pub const WRCTRLD1: u8 = 0x53;
    pub const C4: u8 = 0xC4;
    pub const C63: u8 = 0x63;
}

const CMD_RAMWR: u32 = 0x2C;
const CMD_RAMWRC: u32 = 0x3C;

/// Calculate display offsets at compile time for a given resolution
const fn calculate_offsets_const(
    physical_width: u16,
    physical_height: u16,
    logical_width: u32,
    logical_height: u32,
    scale: u32,
) -> (u16, u16, u16, u16) {
    let used_width = logical_width * scale;
    let used_height = logical_height * scale;
    
    // For scale >= 4, no centering (would exceed physical display with hardware offset)
    let center_x = if scale >= 4 {
        0
    } else {
        let available = (physical_width as u32)
            .saturating_sub(used_width)
            .saturating_sub(HARDWARE_X_OFFSET as u32);
        (available / 2) as u16
    };
    
    let center_y = if scale >= 4 {
        0
    } else {
        let available = (physical_height as u32)
            .saturating_sub(used_height)
            .saturating_sub(HARDWARE_Y_OFFSET as u32);
        (available / 2) as u16
    };
    
    (HARDWARE_X_OFFSET, HARDWARE_Y_OFFSET, center_x, center_y)
}

/// Pre-computed offsets for each resolution mode (hw_x, hw_y, center_x, center_y)
const RESOLUTION_OFFSETS: [(u16, u16, u16, u16); 3] = [
    // Scale 1: 410x502
    calculate_offsets_const(DISPLAY_WIDTH as u16, DISPLAY_HEIGHT as u16, 410, 502, 1),
    // Scale 2: 205x251 -> 410x502
    calculate_offsets_const(DISPLAY_WIDTH as u16, DISPLAY_HEIGHT as u16, 205, 251, 2),
    // Scale 4: 102x125 -> 408x500
    calculate_offsets_const(DISPLAY_WIDTH as u16, DISPLAY_HEIGHT as u16, 102, 125, 4),
];

/// Supported display modes with different scaling factors
const SUPPORTED_RESOLUTIONS: [DisplayResolution; 3] = [
    DisplayResolution {
        logical: DisplaySize::new(410, 502),
        physical: DisplaySize::new(DISPLAY_WIDTH, DISPLAY_HEIGHT),
        scale: 1,
    },
    DisplayResolution {
        logical: DisplaySize::new(205, 251),
        physical: DisplaySize::new(DISPLAY_WIDTH, DISPLAY_HEIGHT),
        scale: 2,
    },
    DisplayResolution {
        logical: DisplaySize::new(102, 125),
        physical: DisplaySize::new(DISPLAY_WIDTH, DISPLAY_HEIGHT),
        scale: 4,
    },
];

const PREFERRED_MODE_INDEX: usize = 2;
const SUPPORTED_FORMATS: [PixelFormat; 1] = [PixelFormat::Rgb565];

// ============================================================================
// Helper Functions
// ============================================================================

/// Convert PixelFormat to SH8601 COLMOD register value
#[inline]
fn pixel_format_to_colmod(fmt: PixelFormat) -> u8 {
    match fmt {
        PixelFormat::Rgb565 => 0x55,
        PixelFormat::Rgb888 => 0x77,
        PixelFormat::Rgb666 => 0x66,
        PixelFormat::Gray8 => 0x11,
    }
}

// ============================================================================
// Display Driver
// ============================================================================

/// Driver for CO5300/SH8601-based AMOLED displays
/// Supports QSPI interface with hardware acceleration and multiple scaling modes
pub struct Co5300<RST> {
    qspi: SpiDmaBus<'static, esp_hal::Async>,
    reset_pin: RST,
    width: u16,
    height: u16,
    hw_x_offset: u16,
    hw_y_offset: u16,
    center_x_offset: u16,
    center_y_offset: u16,
    pixel_format: PixelFormat,
    active_resolution: DisplayResolution,
}

impl<RST> Co5300<RST>
where
    RST: OutputPin,
{
    pub fn new(
        qspi: SpiDmaBus<'static, esp_hal::Async>,
        reset_pin: RST,
        width: u16,
        height: u16,
        pixel_format: PixelFormat,
    ) -> Self {
        debug!("Creating CO5300 driver {}x{}", width, height);
        let active_resolution = SUPPORTED_RESOLUTIONS[PREFERRED_MODE_INDEX];
        let (hw_x_offset, hw_y_offset, center_x_offset, center_y_offset) =
            RESOLUTION_OFFSETS[PREFERRED_MODE_INDEX];

        debug!(
            "Using resolution {}x{} scale {} with offsets HW:({},{}) Center:({},{})",
            active_resolution.logical.width,
            active_resolution.logical.height,
            active_resolution.scale,
            hw_x_offset,
            hw_y_offset,
            center_x_offset,
            center_y_offset
        );

        Self {
            qspi,
            reset_pin,
            width,
            height,
            hw_x_offset,
            hw_y_offset,
            center_x_offset,
            center_y_offset,
            pixel_format,
            active_resolution,
        }
    }

    // ------------------------------------------------------------------------
    // Low-level QSPI Communication
    // ------------------------------------------------------------------------

    #[inline]
    async fn send_command(&mut self, cmd: u8) -> Result<(), esp_hal::spi::Error> {
        self.qspi.half_duplex_write(
            DataMode::Single,
            Command::_8Bit(QSPI_CONTROL_OPCODE as u16, DataMode::Single),
            Address::_24Bit((cmd as u32) << 8, DataMode::Single),
            0,
            &[],
        )
    }

    #[inline]
    async fn send_command_with_data(
        &mut self,
        cmd: u8,
        data: &[u8],
    ) -> Result<(), esp_hal::spi::Error> {
        self.qspi.half_duplex_write(
            DataMode::Single,
            Command::_8Bit(QSPI_CONTROL_OPCODE as u16, DataMode::Single),
            Address::_24Bit((cmd as u32) << 8, DataMode::Single),
            0,
            data,
        )
    }

    async fn send_pixels(&mut self, pixels: &[u8]) -> Result<(), esp_hal::spi::Error> {
        for (index, chunk) in pixels.chunks(DMA_CHUNK_SIZE).enumerate() {
            let addr = if index == 0 { CMD_RAMWR } else { CMD_RAMWRC };
            self.qspi.half_duplex_write(
                DataMode::Quad,
                Command::_8Bit(QSPI_PIXEL_OPCODE as u16, DataMode::Single),
                Address::_24Bit(addr << 8, DataMode::Single),
                0,
                chunk,
            )?;
        }
        Ok(())
    }

    // ------------------------------------------------------------------------
    // Hardware Control
    // ------------------------------------------------------------------------

    async fn hard_reset(&mut self) -> Result<(), ()> {
        debug!("Performing hardware reset");
        self.reset_pin.set_low().map_err(|_| ())?;
        Timer::after(Duration::from_millis(10)).await;
        self.reset_pin.set_high().map_err(|_| ())?;
        Timer::after(Duration::from_millis(150)).await;
        Ok(())
    }

    async fn set_window(
        &mut self,
        x_start: u16,
        y_start: u16,
        x_end: u16,
        y_end: u16,
    ) -> Result<(), esp_hal::spi::Error> {
        // Apply both hardware and centering offsets to both start and end
        // This matches the reference ESP-IDF driver behavior
        let x_start = x_start + self.hw_x_offset + self.center_x_offset;
        let x_end = x_end + self.hw_x_offset + self.center_x_offset;
        let y_start = y_start + self.hw_y_offset + self.center_y_offset;
        let y_end = y_end + self.hw_y_offset + self.center_y_offset;

        self.send_command_with_data(
            commands::CASET,
            &[
                (x_start >> 8) as u8,
                (x_start & 0xFF) as u8,
                ((x_end - 1) >> 8) as u8,
                ((x_end - 1) & 0xFF) as u8,
            ],
        )
        .await?;

        self.send_command_with_data(
            commands::PASET,
            &[
                (y_start >> 8) as u8,
                (y_start & 0xFF) as u8,
                ((y_end - 1) >> 8) as u8,
                ((y_end - 1) & 0xFF) as u8,
            ],
        )
        .await
    }

    /// Set raw window without any offsets - for clearing the entire physical display
    async fn set_raw_window(
        &mut self,
        x_start: u16,
        y_start: u16,
        x_end: u16,
        y_end: u16,
    ) -> Result<(), esp_hal::spi::Error> {
        self.send_command_with_data(
            commands::CASET,
            &[
                (x_start >> 8) as u8,
                (x_start & 0xFF) as u8,
                ((x_end - 1) >> 8) as u8,
                ((x_end - 1) & 0xFF) as u8,
            ],
        )
        .await?;

        self.send_command_with_data(
            commands::PASET,
            &[
                (y_start >> 8) as u8,
                (y_start & 0xFF) as u8,
                ((y_end - 1) >> 8) as u8,
                ((y_end - 1) & 0xFF) as u8,
            ],
        )
        .await
    }

    /// Clear the entire physical display RAM to black (RGB565 = 0x0000)
    /// Uses hardware offset coordinates to match actual drawing
    pub async fn clear_physical_display(&mut self) -> Result<(), esp_hal::spi::Error> {
        debug!("Clearing entire physical display RAM to black");
        
        // Set window with hardware offset: 0x16 to 0x1AF (22 to 431) = 410 pixels
        // This matches how the reference code initializes the display
        self.send_command_with_data(
            commands::CASET,
            &[
                0x00, 0x16,  // Start: 22 (0x16 - hardware offset)
                0x01, 0xAF,  // End: 431 (0x1AF)
            ],
        ).await?;
        
        self.send_command_with_data(
            commands::PASET,
            &[
                0x00, 0x00,  // Start: 0
                0x01, 0xF5,  // End: 501 (0x1F5) 
            ],
        ).await?;
        
        // Fill 410x502 pixels with black
        let total_bytes = 410u32 * 502u32 * 2;
        let total_chunks = ((total_bytes + DMA_CHUNK_SIZE as u32 - 1) / DMA_CHUNK_SIZE as u32) as usize;
        
        debug!("Clearing 410x502 with HW offset ({} chunks)", total_chunks);
        
        let black_chunk = vec![0u8; DMA_CHUNK_SIZE];
        
        for i in 0..total_chunks {
            let cmd = if i == 0 { CMD_RAMWR } else { CMD_RAMWRC };
            
            self.qspi.half_duplex_write(
                DataMode::Quad,
                Command::_8Bit(QSPI_PIXEL_OPCODE as u16, DataMode::Single),
                Address::_24Bit(cmd << 8, DataMode::Single),
                0,
                &black_chunk,
            )?;
        }
        
        debug!("Physical display RAM cleared");
        Ok(())
    }

    /// TEST: Draw checkered pattern to top quarter of screen to test memory retention
    pub async fn test_draw_checkered_pattern(&mut self) -> Result<(), esp_hal::spi::Error> {
        debug!("Drawing checkered pattern to top quarter");
        
        // Top quarter: 410 x 125 pixels
        // Set window WITHOUT hardware offset - controller applies it internally!
        self.send_command_with_data(
            commands::CASET,
            &[
                0x00, 0x00,  // Start: 0
                0x01, 0x99,  // End: 409 (0x199)
            ],
        ).await?;
        
        self.send_command_with_data(
            commands::PASET,
            &[
                0x00, 0x00,  // Start: 0
                0x00, 0x7C,  // End: 124
            ],
        ).await?;
        
        // Create checkered pattern: alternating red (0xF800) and blue (0x001F)
        let pattern_size = 410 * 125 * 2; // 410 pixels wide, 125 pixels tall, 2 bytes per pixel
        let mut pattern = vec![0u8; pattern_size];
        
        for y in 0..125 {
            for x in 0..410 {
                let idx = ((y * 410 + x) * 2) as usize;
                // Checkered pattern: 8x8 blocks
                let color = if ((x / 8) + (y / 8)) % 2 == 0 {
                    0xF800u16  // Red
                } else {
                    0x001Fu16  // Blue
                };
                pattern[idx] = (color >> 8) as u8;
                pattern[idx + 1] = (color & 0xFF) as u8;
            }
        }
        
        debug!("Sending {} bytes of checkered pattern", pattern_size);
        
        // Send pattern in chunks
        for (i, chunk) in pattern.chunks(DMA_CHUNK_SIZE).enumerate() {
            let cmd = if i == 0 { CMD_RAMWR } else { CMD_RAMWRC };
            self.qspi.half_duplex_write(
                DataMode::Quad,
                Command::_8Bit(QSPI_PIXEL_OPCODE as u16, DataMode::Single),
                Address::_24Bit(cmd << 8, DataMode::Single),
                0,
                chunk,
            )?;
        }
        
        debug!("Checkered pattern drawn");
        Ok(())
    }

    // ------------------------------------------------------------------------
    // Scaling Implementations
    // ------------------------------------------------------------------------

    /// Fast path: no scaling, direct buffer transfer
    async fn draw_no_scale(&mut self, buffer: &[u8], region: Rect) {
        let x = region.top_left.x as u16;
        let y = region.top_left.y as u16;
        let w = region.size.width as u16;
        let h = region.size.height as u16;

        if self.set_window(x, y, x + w, y + h).await.is_err() {
            return;
        }

        let t0 = Instant::now();
        if let Err(_) = self.send_pixels(buffer).await {
            error!("Pixel transfer failed (no scale)");
            return;
        }

        debug!(
            "draw_no_scale: {}x{} in {}ms",
            w,
            h,
            t0.elapsed().as_millis()
        );
    }

    /// Generic scaling path for scale factors 2 and other non-optimized scales
    async fn draw_with_scale(&mut self, buffer: &[u8], region: Rect, scale: u16) {
        let x = region.top_left.x as u16;
        let y = region.top_left.y as u16;
        let w = region.size.width as u16;
        let h = region.size.height as u16;

        let scaled_w = w * scale;
        let scaled_h = h * scale;
        let display_x = x * scale;
        let display_y = y * scale;

        let t0 = Instant::now();
        let mut total_scaling = 0u64;
        let mut total_transfer = 0u64;

        for y_chunk in (0..scaled_h).step_by(SCALING_CHUNK_HEIGHT as usize) {
            let chunk_h = core::cmp::min(SCALING_CHUNK_HEIGHT, scaled_h - y_chunk);

            if self
                .set_window(
                    display_x,
                    display_y + y_chunk,
                    display_x + scaled_w,
                    display_y + y_chunk + chunk_h,
                )
                .await
                .is_err()
            {
                return;
            }

            let t_scale = Instant::now();
            let mut chunk = vec![0u8; (scaled_w * chunk_h * 2) as usize];

            // Scale pixels
            for row in 0..chunk_h as usize {
                let src_row = ((row + y_chunk as usize) / scale as usize) * w as usize;
                let dst_row = row * scaled_w as usize;

                for col in 0..scaled_w as usize {
                    let src_col = col / scale as usize;
                    let src_idx = (src_row + src_col) * 2;
                    let dst_idx = (dst_row + col) * 2;
                    chunk[dst_idx] = buffer[src_idx];
                    chunk[dst_idx + 1] = buffer[src_idx + 1];
                }
            }
            total_scaling += t_scale.elapsed().as_micros();

            let t_tx = Instant::now();
            if self.send_pixels(&chunk).await.is_err() {
                error!("Pixel transfer failed (scale {})", scale);
                return;
            }
            total_transfer += t_tx.elapsed().as_micros();
        }

        debug!(
            "draw_scale{}: {}x{} in {}ms (scale:{}ms, tx:{}ms)",
            scale,
            w,
            h,
            t0.elapsed().as_millis(),
            total_scaling / 1000,
            total_transfer / 1000
        );
    }

    /// Optimized scale 4 path using u64 operations for 4x pixel replication
    async fn draw_scale4(&mut self, buffer: &[u8], region: Rect) {
        let x = region.top_left.x as u16;
        let y = region.top_left.y as u16;
        let w = region.size.width as u16;
        let h = region.size.height as u16;

        let scaled_w = w * 4;
        let scaled_h = h * 4;
        let display_x = x * 4;
        let display_y = y * 4;

        let row_u64s = (scaled_w / 4) as usize;
        let mut chunk = vec![0u64; row_u64s * SCALING_CHUNK_HEIGHT as usize];
        let mut scaled_row = vec![0u64; row_u64s];

        let t0 = Instant::now();
        let mut total_scaling = 0u64;
        let mut total_transfer = 0u64;

        for y_chunk in (0..scaled_h).step_by(SCALING_CHUNK_HEIGHT as usize) {
            let chunk_h = core::cmp::min(SCALING_CHUNK_HEIGHT, scaled_h - y_chunk);

            if self
                .set_window(
                    display_x,
                    display_y + y_chunk,
                    display_x + scaled_w,
                    display_y + y_chunk + chunk_h,
                )
                .await
                .is_err()
            {
                return;
            }

            let t_scale = Instant::now();
            unsafe {
                let src_ptr = buffer.as_ptr();
                let mut dst_offset = 0usize;
                let mut cached_src_row = usize::MAX;

                for row in 0..chunk_h as usize {
                    let src_row = (row + y_chunk as usize) / 4;

                    // Cache source row if changed
                    if src_row != cached_src_row {
                        cached_src_row = src_row;
                        let row_ptr = src_ptr.add(src_row * w as usize * 2);

                        for col in 0..w as usize {
                            let pixel =
                                core::ptr::read_unaligned(row_ptr.add(col * 2) as *const u16);
                            // Replicate pixel 4 times in u64
                            scaled_row[col] = (pixel as u64)
                                | ((pixel as u64) << 16)
                                | ((pixel as u64) << 32)
                                | ((pixel as u64) << 48);
                        }
                    }

                    // Copy cached row
                    core::ptr::copy_nonoverlapping(
                        scaled_row.as_ptr(),
                        chunk.as_mut_ptr().add(dst_offset),
                        row_u64s,
                    );
                    dst_offset += row_u64s;
                }
            }
            total_scaling += t_scale.elapsed().as_micros();

            let t_tx = Instant::now();
            let chunk_bytes = unsafe {
                core::slice::from_raw_parts(
                    chunk.as_ptr() as *const u8,
                    row_u64s * chunk_h as usize * 8,
                )
            };

            if self.send_pixels(chunk_bytes).await.is_err() {
                error!("Pixel transfer failed (scale 4)");
                return;
            }
            total_transfer += t_tx.elapsed().as_micros();
        }

        debug!(
            "draw_scale4: {}x{} in {}ms (scale:{}ms, tx:{}ms)",
            w,
            h,
            t0.elapsed().as_millis(),
            total_scaling / 1000,
            total_transfer / 1000
        );
    }
}

// ============================================================================
// AsyncDisplay Implementation
// ============================================================================

#[async_trait(?Send)]
impl<RST> AsyncDisplay for Co5300<RST>
where
    RST: OutputPin + Send,
{
    async fn init(&mut self) {
        debug!("Initializing CO5300 display");

        if self.hard_reset().await.is_err() {
            error!("Hard reset failed");
            return;
        }

        let init = async {
            // Wake from sleep
            self.send_command(commands::SLPOUT).await?;
            Timer::after(Duration::from_millis(80)).await;

            // Vendor-specific initialization sequence
            self.send_command_with_data(commands::C4, &[0x80]).await?;
            self.send_command_with_data(commands::WRCTRLD1, &[0x20])
                .await?;
            Timer::after(Duration::from_millis(1)).await;

            self.send_command_with_data(commands::C63, &[0xFF]).await?;
            Timer::after(Duration::from_millis(1)).await;

            self.send_command_with_data(commands::WRDISBV, &[0x00])
                .await?;
            Timer::after(Duration::from_millis(1)).await;

            self.send_command(commands::DISPON).await?;
            Timer::after(Duration::from_millis(10)).await;

            // Set full brightness
            self.send_command_with_data(commands::WRDISBV, &[0xFF])
                .await?;

            // Configure tearing effect and scan direction
            self.send_command_with_data(commands::TESCAN, &[0x00, 0xC8])
                .await?;
            self.send_command_with_data(commands::TEON, &[0x00]).await?;
            self.send_command_with_data(commands::WRCTRLD1, &[0x20])
                .await?;
            Timer::after(Duration::from_millis(25)).await;

            // Set memory access and pixel format
            self.send_command_with_data(commands::MADCTL, &[0x00])
                .await?;
            self.send_command_with_data(
                commands::COLMOD,
                &[pixel_format_to_colmod(self.pixel_format)],
            )
            .await?;

            // Clear entire display RAM with massive overfill
            self.clear_physical_display().await?;

            // Final display on
            self.send_command(commands::DISPON).await?;

            Ok::<(), esp_hal::spi::Error>(())
        }
        .await;

        match init {
            Ok(_) => debug!("Display initialized successfully"),
            Err(_) => error!("Display initialization failed"),
        }
    }

    async fn set_brightness(&mut self, value: u8) {
        if self
            .send_command_with_data(commands::WRDISBV, &[value])
            .await
            .is_err()
        {
            error!("Failed to set brightness");
        }
    }

    async fn draw_region(&mut self, buffer: &[u8], region: Rect) {
        match self.active_resolution.scale as u16 {
            1 => self.draw_no_scale(buffer, region).await,
            4 => self.draw_scale4(buffer, region).await,
            s => self.draw_with_scale(buffer, region, s).await,
        }
    }

    async fn draw(&mut self, buffer: &[u8]) {
        let full = Rect::new(
            Point::new(0, 0),
            Size::new(
                self.active_resolution.logical.width,
                self.active_resolution.logical.height,
            ),
        );
        self.draw_region(buffer, full).await;
    }

    async fn paint_screen(&mut self, _color: u8) {
        // Test pattern implementation (optional)
    }

    async fn set_orientation(&mut self, _orientation: Orientation) {
        // Not currently implemented
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
        DisplayCapabilities {
            supported_formats: &SUPPORTED_FORMATS,
            preferred_format: PixelFormat::Rgb565,
            supported_resolutions: &SUPPORTED_RESOLUTIONS,
            preferred_resolution: SUPPORTED_RESOLUTIONS[PREFERRED_MODE_INDEX],
        }
    }

    fn set_resolution(&mut self, resolution: DisplayResolution) {
        let caps = self.capabilities();
        let selected = caps
            .supported_resolutions
            .iter()
            .copied()
            .find(|m| m.logical == resolution.logical && m.scale == resolution.scale)
            .or_else(|| {
                caps.supported_resolutions
                    .iter()
                    .copied()
                    .find(|m| m.scale == resolution.scale)
            })
            .unwrap_or(caps.preferred_resolution);

        self.active_resolution = selected;
        
        // Find the index of the selected resolution to get pre-computed offsets
        let mode_index = SUPPORTED_RESOLUTIONS.iter()
            .position(|r| r.logical == selected.logical && r.scale == selected.scale)
            .unwrap_or(PREFERRED_MODE_INDEX);
        
        let (hw_x, hw_y, center_x, center_y) = RESOLUTION_OFFSETS[mode_index];
        self.hw_x_offset = hw_x;
        self.hw_y_offset = hw_y;
        self.center_x_offset = center_x;
        self.center_y_offset = center_y;

        debug!(
            "Resolution set to {}x{} @ {}x scale",
            selected.logical.width, selected.logical.height, selected.scale
        );
    }
}
