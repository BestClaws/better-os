//! CO5300/SH8601 AMOLED Display Driver
//!
//! This driver supports the CO5300 display module with SH8601 controller,
//! featuring QSPI interface, hardware-accelerated scaling, and multiple resolution modes.
//!
//! # Display Specifications
//! - Physical Resolution: 410×502 pixels
//! - Interface: QSPI (Quad-SPI)
//! - Pixel Format: RGB565 (primary), RGB888, RGB666, Gray8
//! - Hardware Offset: 22 pixels X-axis (built into controller)
//!
//! # Scaling Modes
//! The driver supports integer scaling factors (1x, 2x, 4x) to reduce memory usage
//! while maintaining crisp pixel-perfect rendering:
//! - **1x (410×502)**: Full resolution, no scaling
//! - **2x (205×251)**: Half resolution, 2x2 pixel blocks
//! - **4x (102×125)**: Quarter resolution, 4x4 pixel blocks (default, optimized)
//!
//! # Architecture
//! ```text
//! ┌─────────────────────────────────────┐
//! │   AsyncDisplay Trait (High-Level)   │
//! ├─────────────────────────────────────┤
//! │  Drawing Operations                 │
//! │  • draw() / draw_region()           │
//! │  • Scaling dispatch (1x/2x/4x)      │
//! ├─────────────────────────────────────┤
//! │  Display Control (Mid-Level)        │
//! │  • Window management                │
//! │  • Offset calculations              │
//! │  • Clear operations                 │
//! ├─────────────────────────────────────┤
//! │  Protocol Layer (Low-Level)         │
//! │  • QSPI command/data transfer       │
//! │  • DMA chunking                     │
//! │  • Hardware reset                   │
//! └─────────────────────────────────────┘
//! ```

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

// ═══════════════════════════════════════════════════════════════════════════
// Hardware Configuration & Constants
// ═══════════════════════════════════════════════════════════════════════════

/// Hardware X-axis offset built into the SH8601 controller.
///
/// The display RAM is larger than the visible screen. The visible area starts
/// at RAM coordinate (22, 0), not (0, 0). This offset must be added to all
/// column (X) coordinates when setting display windows.
///
/// From reference implementation: CASET 0x0016 to 0x01AF (22 to 431)
const HARDWARE_X_OFFSET: u16 = 0x16; // 22 pixels

/// Hardware Y-axis offset (currently 0 for this display)
const HARDWARE_Y_OFFSET: u16 = 0x00; // 0 pixels

/// QSPI opcode for pixel data transfer (Quad-SPI mode)
const QSPI_PIXEL_OPCODE: u8 = 0x32;

/// QSPI opcode for command/control data transfer (Single-SPI mode)
const QSPI_CONTROL_OPCODE: u8 = 0x02;

/// DMA transfer chunk size in bytes.
///
/// Limited by DMA buffer constraints. Transfers are automatically chunked
/// to avoid exceeding hardware limits while maintaining high throughput.
const DMA_CHUNK_SIZE: usize = 16380; // ~16KB

/// Height of each scaling chunk when processing scaled frames.
///
/// Larger values use more memory but reduce overhead. Smaller values
/// use less memory but increase loop/DMA overhead.
const SCALING_CHUNK_HEIGHT: u16 = 50;

/// SH8601 display controller command set
///
/// These commands control the display hardware. Commands are sent via
/// Single-SPI mode, while pixel data uses Quad-SPI for higher bandwidth.
pub mod commands {
    pub const NOP: u8 = 0x00;          // No operation
    pub const SWRESET: u8 = 0x01;      // Software reset
    pub const SLPIN: u8 = 0x10;        // Enter sleep mode
    pub const SLPOUT: u8 = 0x11;       // Exit sleep mode
    pub const INVOFF: u8 = 0x20;       // Display inversion off
    pub const INVON: u8 = 0x21;        // Display inversion on
    pub const DISPOFF: u8 = 0x28;      // Display off
    pub const DISPON: u8 = 0x29;       // Display on
    pub const CASET: u8 = 0x2A;        // Column address set (X coordinates)
    pub const PASET: u8 = 0x2B;        // Page address set (Y coordinates)
    pub const RAMWR: u8 = 0x2C;        // Memory write (start new transfer)
    pub const RAMWRC: u8 = 0x3C;       // Memory write continue (continue transfer)
    pub const TEON: u8 = 0x35;         // Tearing effect line on
    pub const MADCTL: u8 = 0x36;       // Memory data access control
    pub const COLMOD: u8 = 0x3A;       // Pixel format set
    pub const TESCAN: u8 = 0x44;       // Tearing effect scan line
    pub const WRDISBV: u8 = 0x51;      // Write display brightness
    pub const WRCTRLD1: u8 = 0x53;     // Write control display
    pub const C4: u8 = 0xC4;           // Vendor-specific command
    pub const C63: u8 = 0x63;          // Vendor-specific command
}

/// RAMWR command code for first chunk (starts new write)
const CMD_RAMWR: u32 = 0x2C;

/// RAMWRC command code for subsequent chunks (continues write)
const CMD_RAMWRC: u32 = 0x3C;

// ═══════════════════════════════════════════════════════════════════════════
// Resolution Configuration & Offset Calculation
// ═══════════════════════════════════════════════════════════════════════════

/// Calculate display window offsets at compile time for a given resolution mode.
///
/// # Offset Components
/// 1. **Hardware Offset**: Built into SH8601 controller (22, 0)
/// 2. **Centering Offset**: Centers scaled content within physical display
///
/// # Returns
/// Tuple of (hw_x_offset, hw_y_offset, center_x_offset, center_y_offset)
///
/// # Example
/// For scale 4 (102×125 logical → 408×500 scaled on 410×502 physical):
/// - Scaled size: 408×500
/// - Physical: 410×502
/// - Remaining space: 2 pixels X, 2 pixels Y
/// - Centering: 1 pixel offset X, 1 pixel offset Y
const fn calculate_offsets_const(
    physical_width: u16,
    physical_height: u16,
    logical_width: u32,
    logical_height: u32,
    scale: u32,
) -> (u16, u16, u16, u16) {
    let scaled_width = logical_width * scale;
    let scaled_height = logical_height * scale;
    
    // For scale >= 4, skip centering to avoid exceeding physical bounds
    // with hardware offset applied (408 + 22 + 1 would exceed 410)
    let center_x = if scale >= 4 {
        0
    } else {
        let available_space = (physical_width as u32)
            .saturating_sub(scaled_width)
            .saturating_sub(HARDWARE_X_OFFSET as u32);
        (available_space / 2) as u16
    };
    
    let center_y = if scale >= 4 {
        0
    } else {
        let available_space = (physical_height as u32)
            .saturating_sub(scaled_height)
            .saturating_sub(HARDWARE_Y_OFFSET as u32);
        (available_space / 2) as u16
    };
    
    (HARDWARE_X_OFFSET, HARDWARE_Y_OFFSET, center_x, center_y)
}

/// Pre-computed offsets for each supported resolution mode.
///
/// Calculated at compile time using const fn, zero runtime cost.
/// Each entry: (hardware_x, hardware_y, centering_x, centering_y)
const RESOLUTION_OFFSETS: [(u16, u16, u16, u16); 3] = [
    // Scale 1: 410×502 (full resolution, no scaling)
    calculate_offsets_const(DISPLAY_WIDTH as u16, DISPLAY_HEIGHT as u16, 410, 502, 1),
    
    // Scale 2: 205×251 → 410×502 (2x2 blocks)
    calculate_offsets_const(DISPLAY_WIDTH as u16, DISPLAY_HEIGHT as u16, 205, 251, 2),
    
    // Scale 4: 102×125 → 408×500 (4x4 blocks, optimized, default)
    calculate_offsets_const(DISPLAY_WIDTH as u16, DISPLAY_HEIGHT as u16, 102, 125, 4),
];

/// Supported display resolution modes
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

/// Default resolution mode (index into SUPPORTED_RESOLUTIONS)
///
/// Mode 2 = 102×125 @ 4x scale (uses ~25KB instead of ~410KB for framebuffer)
const PREFERRED_MODE_INDEX: usize = 2;

/// Supported pixel formats (currently only RGB565 fully tested)
const SUPPORTED_FORMATS: [PixelFormat; 1] = [PixelFormat::Rgb565];

// ═══════════════════════════════════════════════════════════════════════════
// Helper Functions
// ═══════════════════════════════════════════════════════════════════════════

/// Convert PixelFormat enum to SH8601 COLMOD register value
#[inline]
fn pixel_format_to_colmod(fmt: PixelFormat) -> u8 {
    match fmt {
        PixelFormat::Rgb565 => 0x55, // 16-bit/pixel RGB565
        PixelFormat::Rgb888 => 0x77, // 24-bit/pixel RGB888
        PixelFormat::Rgb666 => 0x66, // 18-bit/pixel RGB666
        PixelFormat::Gray8 => 0x11,  // 8-bit/pixel grayscale
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// Display Driver Implementation
// ═══════════════════════════════════════════════════════════════════════════

/// CO5300/SH8601 AMOLED display driver
///
/// # Type Parameters
/// - `RST`: GPIO pin type for hardware reset (must implement OutputPin)
///
/// # Fields Organization
/// - **qspi**: DMA-enabled SPI bus for high-speed data transfer
/// - **reset_pin**: GPIO for hardware reset
/// - **width/height**: Physical display dimensions
/// - **offsets**: Pre-computed coordinate offsets
/// - **pixel_format**: Active color format
/// - **active_resolution**: Current resolution mode
pub struct Co5300<RST> {
    // Hardware interfaces
    qspi: SpiDmaBus<'static, esp_hal::Async>,
    reset_pin: RST,
    
    // Display dimensions
    width: u16,
    height: u16,
    
    // Coordinate offsets (applied to all drawing operations)
    hw_x_offset: u16,
    hw_y_offset: u16,
    center_x_offset: u16,
    center_y_offset: u16,
    
    // Display configuration
    pixel_format: PixelFormat,
    active_resolution: DisplayResolution,
}

impl<RST> Co5300<RST>
where
    RST: OutputPin,
{
    /// Create a new CO5300 display driver instance
    ///
    /// # Parameters
    /// - `qspi`: DMA-enabled QSPI bus
    /// - `reset_pin`: GPIO pin for hardware reset
    /// - `width`: Physical display width (410)
    /// - `height`: Physical display height (502)
    /// - `pixel_format`: Pixel format (typically RGB565)
    pub fn new(
        qspi: SpiDmaBus<'static, esp_hal::Async>,
        reset_pin: RST,
        width: u16,
        height: u16,
        pixel_format: PixelFormat,
    ) -> Self {
        debug!("Creating CO5300 driver {}×{}", width, height);
        
        let active_resolution = SUPPORTED_RESOLUTIONS[PREFERRED_MODE_INDEX];
        let (hw_x_offset, hw_y_offset, center_x_offset, center_y_offset) =
            RESOLUTION_OFFSETS[PREFERRED_MODE_INDEX];

        debug!(
            "Resolution: {}×{} @ {}× scale, Offsets: HW({},{}) Center({},{})",
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

    // ═══════════════════════════════════════════════════════════════════════
    // Low-Level Protocol Layer
    // ═══════════════════════════════════════════════════════════════════════

    /// Send a command without data
    ///
    /// Commands are sent via Single-SPI mode using the control opcode.
    /// The command byte is placed in the address field (shifted left 8 bits).
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

    /// Send a command with associated data
    ///
    /// Similar to send_command() but includes a data payload.
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

    /// Send pixel data to display RAM
    ///
    /// Automatically chunks large transfers to stay within DMA limits.
    /// First chunk uses RAMWR, subsequent chunks use RAMWRC.
    ///
    /// # Protocol Details
    /// - Uses Quad-SPI mode (4x bandwidth vs Single-SPI)
    /// - RAMWR: Starts a new write sequence
    /// - RAMWRC: Continues the previous write sequence
    async fn send_pixels(&mut self, pixels: &[u8]) -> Result<(), esp_hal::spi::Error> {
        for (index, chunk) in pixels.chunks(DMA_CHUNK_SIZE).enumerate() {
            let cmd = if index == 0 { CMD_RAMWR } else { CMD_RAMWRC };
            
            self.qspi.half_duplex_write(
                DataMode::Quad,
                Command::_8Bit(QSPI_PIXEL_OPCODE as u16, DataMode::Single),
                Address::_24Bit(cmd << 8, DataMode::Single),
                0,
                chunk,
            )?;
        }
        Ok(())
    }

    // ═══════════════════════════════════════════════════════════════════════
    // Display Control Layer
    // ═══════════════════════════════════════════════════════════════════════

    /// Perform hardware reset sequence
    ///
    /// Timing requirements:
    /// - Hold low for 10ms
    /// - Wait 150ms after releasing
    async fn hardware_reset(&mut self) -> Result<(), ()> {
        debug!("Hardware reset");
        self.reset_pin.set_low().map_err(|_| ())?;
        Timer::after(Duration::from_millis(10)).await;
        self.reset_pin.set_high().map_err(|_| ())?;
        Timer::after(Duration::from_millis(150)).await;
        Ok(())
    }

    /// Set the display window (drawing region) with offset application
    ///
    /// This is the primary windowing function used for all drawing operations.
    ///
    /// # Coordinate System
    /// Input coordinates are in logical space (0-based). This function:
    /// 1. Adds hardware offset (22, 0)
    /// 2. Adds centering offset (if any)
    /// 3. Sends CASET/PASET commands with adjusted coordinates
    ///
    /// # Parameters
    /// - `x_start`, `y_start`: Top-left corner (inclusive)
    /// - `x_end`, `y_end`: Bottom-right corner (exclusive, typical convention)
    ///
    /// # Why Offsets Are Added to Both Start and End
    /// The offset shifts the entire window, not just the starting position.
    /// This matches the reference ESP-IDF implementation.
    async fn set_window(
        &mut self,
        x_start: u16,
        y_start: u16,
        x_end: u16,
        y_end: u16,
    ) -> Result<(), esp_hal::spi::Error> {
        // Apply offsets to shift the window to the correct RAM location
        let x_start_adj = x_start + self.hw_x_offset + self.center_x_offset;
        let x_end_adj = x_end + self.hw_x_offset + self.center_x_offset;
        let y_start_adj = y_start + self.hw_y_offset + self.center_y_offset;
        let y_end_adj = y_end + self.hw_y_offset + self.center_y_offset;

        // Set column address (X coordinates)
        self.send_command_with_data(
            commands::CASET,
            &[
                (x_start_adj >> 8) as u8,
                (x_start_adj & 0xFF) as u8,
                ((x_end_adj - 1) >> 8) as u8,      // -1 because end is inclusive
                ((x_end_adj - 1) & 0xFF) as u8,
            ],
        )
        .await?;

        // Set page address (Y coordinates)
        self.send_command_with_data(
            commands::PASET,
            &[
                (y_start_adj >> 8) as u8,
                (y_start_adj & 0xFF) as u8,
                ((y_end_adj - 1) >> 8) as u8,      // -1 because end is inclusive
                ((y_end_adj - 1) & 0xFF) as u8,
            ],
        )
        .await
    }

    /// Clear the entire physical display RAM to black
    ///
    /// This function must use the hardware offset coordinates to address the
    /// correct visible RAM region. The SH8601 controller's RAM is larger than
    /// the visible area, and the visible region starts at offset (22, 0).
    ///
    /// # Why Hardware Offset Matters Here
    /// When we tried clearing from (0, 0), we were clearing an invisible region
    /// of RAM. The visible area is at (22, 0) to (431, 501), so we must clear
    /// starting from those coordinates.
    ///
    /// # Memory Retention
    /// The display RAM is persistent. Pixels cleared to black stay black even
    /// when subsequent draws don't cover them. This is why we can draw 408×500
    /// content and the extra 2 pixels on right/bottom stay black from this clear.
    async fn clear_display_ram(&mut self) -> Result<(), esp_hal::spi::Error> {
        debug!("Clearing display RAM to black");
        
        // Set window to full visible area using hardware offset
        // CASET: 0x16 to 0x1AF (22 to 431) = 410 pixels wide
        // PASET: 0x00 to 0x1F5 (0 to 501) = 502 pixels tall
        self.send_command_with_data(
            commands::CASET,
            &[
                0x00, 0x16,  // Start: 22 (HARDWARE_X_OFFSET)
                0x01, 0xAF,  // End: 431
            ],
        ).await?;
        
        self.send_command_with_data(
            commands::PASET,
            &[
                0x00, 0x00,  // Start: 0
                0x01, 0xF5,  // End: 501
            ],
        ).await?;
        
        // Calculate total bytes: 410 × 502 × 2 bytes/pixel = 411,640 bytes
        let total_bytes = 410u32 * 502u32 * 2;
        let total_chunks = ((total_bytes + DMA_CHUNK_SIZE as u32 - 1) / DMA_CHUNK_SIZE as u32) as usize;
        
        debug!("Clearing 410×502 pixels ({} chunks)", total_chunks);
        
        // Fill with black pixels (RGB565 black = 0x0000)
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
        
        debug!("Display RAM cleared");
        Ok(())
    }

    // ═══════════════════════════════════════════════════════════════════════
    // Scaling Implementation Layer
    // ═══════════════════════════════════════════════════════════════════════

    /// No scaling path (1:1 pixel transfer)
    ///
    /// Directly transfers the buffer to display without any processing.
    /// This is the fastest path but uses the most memory (410×502×2 = ~410KB).
    async fn draw_unscaled(&mut self, buffer: &[u8], region: Rect) {
        let x = region.top_left.x as u16;
        let y = region.top_left.y as u16;
        let w = region.size.width as u16;
        let h = region.size.height as u16;

        if self.set_window(x, y, x + w, y + h).await.is_err() {
            error!("set_window failed");
            return;
        }

        let t0 = Instant::now();
        if self.send_pixels(buffer).await.is_err() {
            error!("Pixel transfer failed (unscaled)");
            return;
        }

        debug!("draw_unscaled: {}×{} in {}ms", w, h, t0.elapsed().as_millis());
    }

    /// Generic scaling path for 2× and other integer scales
    ///
    /// Implements nearest-neighbor scaling by replicating pixels.
    /// Processes the image in horizontal strips (chunks) to limit memory usage.
    ///
    /// # Algorithm
    /// For each output pixel (x, y):
    /// - Source pixel = (x / scale, y / scale)
    /// - Replicate source pixel value to output
    ///
    /// # Memory Usage
    /// Processes SCALING_CHUNK_HEIGHT rows at a time:
    /// - 2× scale, 205×50 chunk = 205 × 50 × 4 × 2 bytes = ~80KB
    async fn draw_scaled_generic(&mut self, buffer: &[u8], region: Rect, scale: u16) {
        let x = region.top_left.x as u16;
        let y = region.top_left.y as u16;
        let w = region.size.width as u16;
        let h = region.size.height as u16;

        let scaled_w = w * scale;
        let scaled_h = h * scale;
        let display_x = x * scale;
        let display_y = y * scale;

        let t0 = Instant::now();
        let mut total_scaling_us = 0u64;
        let mut total_transfer_us = 0u64;

        // Process in horizontal strips
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
                error!("set_window failed");
                return;
            }

            // Scale pixels for this chunk
            let t_scale = Instant::now();
            let mut chunk = vec![0u8; (scaled_w * chunk_h * 2) as usize];

            for row in 0..chunk_h as usize {
                let src_row = ((row + y_chunk as usize) / scale as usize) * w as usize;
                let dst_row = row * scaled_w as usize;

                for col in 0..scaled_w as usize {
                    let src_col = col / scale as usize;
                    let src_idx = (src_row + src_col) * 2;
                    let dst_idx = (dst_row + col) * 2;
                    
                    // Copy RGB565 pixel (2 bytes)
                    chunk[dst_idx] = buffer[src_idx];
                    chunk[dst_idx + 1] = buffer[src_idx + 1];
                }
            }
            total_scaling_us += t_scale.elapsed().as_micros();

            // Transfer scaled chunk
            let t_tx = Instant::now();
            if self.send_pixels(&chunk).await.is_err() {
                error!("Pixel transfer failed (scale {})", scale);
                return;
            }
            total_transfer_us += t_tx.elapsed().as_micros();
        }

        debug!(
            "draw_scale{}: {}×{} in {}ms (scale:{}ms, tx:{}ms)",
            scale,
            w,
            h,
            t0.elapsed().as_millis(),
            total_scaling_us / 1000,
            total_transfer_us / 1000
        );
    }

    /// Optimized 4× scaling using u64 operations
    ///
    /// This is the most optimized path, using wide (u64) operations to
    /// replicate 4 RGB565 pixels simultaneously.
    ///
    /// # Optimization Strategy
    /// 1. Pack 4 RGB565 pixels (16-bit each) into a single u64
    /// 2. Replicate source rows 4 times (cache for vertical scaling)
    /// 3. Use SIMD-friendly memory operations (copy_nonoverlapping)
    ///
    /// # Performance
    /// Approximately 3-4× faster than generic scaling due to:
    /// - Single u64 operation replaces 8 byte copies
    /// - Row caching eliminates redundant computations
    /// - Better memory access patterns for CPU cache
    async fn draw_scaled_4x_optimized(&mut self, buffer: &[u8], region: Rect) {
        let x = region.top_left.x as u16;
        let y = region.top_left.y as u16;
        let w = region.size.width as u16;
        let h = region.size.height as u16;

        let scaled_w = w * 4;
        let scaled_h = h * 4;
        let display_x = x * 4;
        let display_y = y * 4;

        // Each u64 holds 4 RGB565 pixels
        let pixels_per_u64 = 4;
        let u64s_per_row = (scaled_w / pixels_per_u64) as usize;
        
        let mut chunk = vec![0u64; u64s_per_row * SCALING_CHUNK_HEIGHT as usize];
        let mut scaled_row_cache = vec![0u64; u64s_per_row];

        let t0 = Instant::now();
        let mut total_scaling_us = 0u64;
        let mut total_transfer_us = 0u64;

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
                error!("set_window failed");
                return;
            }

            let t_scale = Instant::now();
            unsafe {
                let src_ptr = buffer.as_ptr();
                let mut dst_offset = 0usize;
                let mut cached_src_row = usize::MAX;

                for row in 0..chunk_h as usize {
                    let src_row = (row + y_chunk as usize) / 4;

                    // Cache the source row if it changed
                    if src_row != cached_src_row {
                        cached_src_row = src_row;
                        let row_ptr = src_ptr.add(src_row * w as usize * 2);

                        // Build scaled row: replicate each pixel 4 times horizontally
                        for col in 0..w as usize {
                            let pixel = core::ptr::read_unaligned(
                                row_ptr.add(col * 2) as *const u16
                            );
                            
                            // Pack 4 copies of the pixel into u64
                            // Layout: [pixel][pixel][pixel][pixel]
                            scaled_row_cache[col] = (pixel as u64)
                                | ((pixel as u64) << 16)
                                | ((pixel as u64) << 32)
                                | ((pixel as u64) << 48);
                        }
                    }

                    // Copy cached row (vertical replication)
                    core::ptr::copy_nonoverlapping(
                        scaled_row_cache.as_ptr(),
                        chunk.as_mut_ptr().add(dst_offset),
                        u64s_per_row,
                    );
                    dst_offset += u64s_per_row;
                }
            }
            total_scaling_us += t_scale.elapsed().as_micros();

            // Transfer scaled chunk
            let t_tx = Instant::now();
            let chunk_bytes = unsafe {
                core::slice::from_raw_parts(
                    chunk.as_ptr() as *const u8,
                    u64s_per_row * chunk_h as usize * 8,
                )
            };

            if self.send_pixels(chunk_bytes).await.is_err() {
                error!("Pixel transfer failed (scale 4)");
                return;
            }
            total_transfer_us += t_tx.elapsed().as_micros();
        }

        debug!(
            "draw_scale4: {}×{} in {}ms (scale:{}ms, tx:{}ms)",
            w,
            h,
            t0.elapsed().as_millis(),
            total_scaling_us / 1000,
            total_transfer_us / 1000
        );
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// AsyncDisplay Trait Implementation (High-Level Interface)
// ═══════════════════════════════════════════════════════════════════════════

#[async_trait(?Send)]
impl<RST> AsyncDisplay for Co5300<RST>
where
    RST: OutputPin + Send,
{
    /// Initialize the display hardware
    ///
    /// # Initialization Sequence
    /// 1. Hardware reset (GPIO)
    /// 2. Exit sleep mode
    /// 3. Vendor-specific initialization
    /// 4. Configure pixel format
    /// 5. Clear display RAM to black
    /// 6. Enable display output
    async fn init(&mut self) {
        debug!("Initializing CO5300 display");

        if self.hardware_reset().await.is_err() {
            error!("Hardware reset failed");
            return;
        }

        let init_result = async {
            // Wake from sleep mode
            self.send_command(commands::SLPOUT).await?;
            Timer::after(Duration::from_millis(80)).await;

            // Vendor-specific initialization sequence
            self.send_command_with_data(commands::C4, &[0x80]).await?;
            self.send_command_with_data(commands::WRCTRLD1, &[0x20]).await?;
            Timer::after(Duration::from_millis(1)).await;

            self.send_command_with_data(commands::C63, &[0xFF]).await?;
            Timer::after(Duration::from_millis(1)).await;

            self.send_command_with_data(commands::WRDISBV, &[0x00]).await?;
            Timer::after(Duration::from_millis(1)).await;

            // Enable display
            self.send_command(commands::DISPON).await?;
            Timer::after(Duration::from_millis(10)).await;

            // Set full brightness
            self.send_command_with_data(commands::WRDISBV, &[0xFF]).await?;

            // Configure tearing effect (for VSYNC synchronization)
            self.send_command_with_data(commands::TESCAN, &[0x00, 0xC8]).await?;
            self.send_command_with_data(commands::TEON, &[0x00]).await?;
            self.send_command_with_data(commands::WRCTRLD1, &[0x20]).await?;
            Timer::after(Duration::from_millis(25)).await;

            // Set memory access control and pixel format
            self.send_command_with_data(commands::MADCTL, &[0x00]).await?;
            self.send_command_with_data(
                commands::COLMOD,
                &[pixel_format_to_colmod(self.pixel_format)],
            )
            .await?;

            // Clear entire display RAM to eliminate uninitialized pixels
            self.clear_display_ram().await?;

            // Final display enable
            self.send_command(commands::DISPON).await?;

            Ok::<(), esp_hal::spi::Error>(())
        }
        .await;

        match init_result {
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

    /// Draw a region of the framebuffer to the display
    ///
    /// Automatically selects the optimal rendering path based on scale factor:
    /// - Scale 1: Direct transfer (fastest, most memory)
    /// - Scale 4: Optimized u64-based scaling (default)
    /// - Other: Generic nearest-neighbor scaling
    async fn draw_region(&mut self, buffer: &[u8], region: Rect) {
        match self.active_resolution.scale as u16 {
            1 => self.draw_unscaled(buffer, region).await,
            4 => self.draw_scaled_4x_optimized(buffer, region).await,
            scale => self.draw_scaled_generic(buffer, region, scale).await,
        }
    }

    /// Draw the entire framebuffer to the display
    async fn draw(&mut self, buffer: &[u8]) {
        let full_screen = Rect::new(
            Point::new(0, 0),
            Size::new(
                self.active_resolution.logical.width,
                self.active_resolution.logical.height,
            ),
        );
        self.draw_region(buffer, full_screen).await;
    }

    async fn paint_screen(&mut self, _color: u8) {
        // Could be implemented to fill screen with solid color
    }

    async fn set_orientation(&mut self, _orientation: Orientation) {
        // Could be implemented via MADCTL register
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

    /// Switch to a different resolution mode
    ///
    /// Updates active resolution and recalculates offsets from pre-computed table.
    fn set_resolution(&mut self, resolution: DisplayResolution) {
        let caps = self.capabilities();
        
        // Find exact match or fallback to same scale factor
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
        
        // Look up pre-computed offsets
        let mode_index = SUPPORTED_RESOLUTIONS
            .iter()
            .position(|r| r.logical == selected.logical && r.scale == selected.scale)
            .unwrap_or(PREFERRED_MODE_INDEX);
        
        let (hw_x, hw_y, center_x, center_y) = RESOLUTION_OFFSETS[mode_index];
        self.hw_x_offset = hw_x;
        self.hw_y_offset = hw_y;
        self.center_x_offset = center_x;
        self.center_y_offset = center_y;

        debug!(
            "Resolution changed to {}×{} @ {}× scale",
            selected.logical.width, selected.logical.height, selected.scale
        );
    }
}
