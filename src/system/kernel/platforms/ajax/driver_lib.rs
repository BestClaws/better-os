#![no_std]
#[cfg(feature = "waveshare_18_amoled")]
pub mod displays;

#[cfg(feature = "waveshare_18_amoled")]
pub use displays::waveshare_18_amoled::*;

extern crate alloc;

use alloc::boxed::Box;
use embedded_graphics_core::draw_target::DrawTarget;
use embedded_hal::delay::DelayNs;
use embedded_hal_async::delay::DelayNs as AsyncDelayNs;
use defmt::info;

/// Configuration for the display dimensions.
#[derive(Debug, Clone, Copy)]
pub struct DisplaySize {
    pub width: u16,
    pub height: u16,
}

impl DisplaySize {
    pub const fn new(width: u16, height: u16) -> Self {
        DisplaySize { width, height }
    }
}

/// SH8601 Driver Errors
#[derive(Debug)]
pub enum DriverError<InterfaceError, ResetError> {
    InterfaceError(InterfaceError),
    ResetError(ResetError),
    InvalidConfiguration(&'static str),
}

/// Trait to implement the SH8601 controller communication interface (QSPI, SPI, etc.).
pub trait ControllerInterface {
    type Error;

    fn send_command(&mut self, cmd: u8) -> Result<(), Self::Error>;

    fn send_command_with_data(&mut self, cmd: u8, data: &[u8]) -> Result<(), Self::Error>;

    fn send_pixels(&mut self, pixels: &[u8]) -> Result<(), Self::Error>;
}

/// Trait for controlling the SH8601 hardware reset pin.
pub trait ResetInterface {
    type Error;

    fn reset(&mut self) -> Result<(), Self::Error>;
}

/// SH8601 Command Set
pub mod commands {
    pub const NOP: u8 = 0x00;
    pub const SWRESET: u8 = 0x01;
    pub const RDDIDIF: u8 = 0x04;
    pub const RDDPM: u8 = 0x0A;
    pub const RDDMADCTL: u8 = 0x0B;
    pub const RDDCOLMOD: u8 = 0x0C;
    pub const RDDSDR: u8 = 0x0F;
    pub const SLPIN: u8 = 0x10;
    pub const SLPOUT: u8 = 0x11;
    pub const PTLON: u8 = 0x12;
    pub const NORON: u8 = 0x13;
    pub const INVOFF: u8 = 0x20;
    pub const INVON: u8 = 0x21;
    pub const DISPOFF: u8 = 0x28;
    pub const DISPON: u8 = 0x29;
    pub const CASET: u8 = 0x2A;
    pub const PASET: u8 = 0x2B;
    pub const RAMWR: u8 = 0x2C;
    pub const PTLAR: u8 = 0x30;
    pub const TEOFF: u8 = 0x34;
    pub const TEON: u8 = 0x35;
    pub const MADCTL: u8 = 0x36;
    pub const IDMOFF: u8 = 0x38;
    pub const IDMON: u8 = 0x39;
    pub const COLMOD: u8 = 0x3A;
    pub const RAMWRC: u8 = 0x3C;
    pub const TESCAN: u8 = 0x44;
    pub const WRDISBV: u8 = 0x51;
    pub const RDDISBV: u8 = 0x52;
    pub const WRCTRLD1: u8 = 0x53;
    pub const RDCTRLD1: u8 = 0x54;
    pub const C4: u8 = 0xC4;
    pub const C63: u8 = 0x63;
}

/// Color modes supported by the SH8601 display controller.
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

/// Main Driver for the SH8601 display controller.
pub struct Sh8601Driver<IFACE, RST>
where
    IFACE: ControllerInterface,
    RST: ResetInterface,
{
    interface: IFACE,
    reset: RST,
    pub(crate) config: DisplaySize,
    x_gap: u16,
    y_gap: u16,
}

impl<IFACE, RST> Sh8601Driver<IFACE, RST>
where
    IFACE: ControllerInterface,
    RST: ResetInterface,
{
    pub fn new<DELAY>(
        interface: IFACE,
        reset: RST,
        color: ColorMode,
        config: DisplaySize,
        mut delay: DELAY,
    ) -> Result<Self, DriverError<IFACE::Error, RST::Error>>
    where
        DELAY: DelayNs,
    {
        info!("Creating SH8601 driver with color mode");
        let mut driver = Self {
            interface,
            reset,
            config,
            x_gap: 0,
            y_gap: 0,
        };
        driver.hard_reset()?;
        driver.initialize_display(&mut delay, color)?;
        Ok(driver)
    }

    pub fn hard_reset(&mut self) -> Result<(), DriverError<IFACE::Error, RST::Error>> {
        info!("Performing hard reset");
        self.reset.reset().map_err(DriverError::ResetError)?;
        Ok(())
    }

    pub fn initialize_display<DELAY>(
        &mut self,
        delay: &mut DELAY,
        color: ColorMode,
    ) -> Result<(), DriverError<IFACE::Error, RST::Error>>
    where
        DELAY: DelayNs,
    {
        info!("Initializing display with color mode");
        // Initialization sequence from C code
        self.send_command(commands::SLPOUT)?;
        delay.delay_ms(80);

        self.send_command_with_data(commands::C4, &[0x80])?;
        self.send_command_with_data(commands::WRCTRLD1, &[0x20])?;
        delay.delay_ms(1);
        self.send_command_with_data(commands::C63, &[0xFF])?;
        delay.delay_ms(1);
        self.send_command_with_data(commands::WRDISBV, &[0x00])?;
        delay.delay_ms(1);
        self.send_command(commands::DISPON)?;
        delay.delay_ms(10);
        self.send_command_with_data(commands::WRDISBV, &[0xFF])?;

        // Vendor-specific initialization
        self.send_command_with_data(commands::TESCAN, &[0x00, 0xC8])?;
        self.send_command_with_data(commands::TEON, &[0x00])?;
        self.send_command_with_data(commands::WRCTRLD1, &[0x20])?;
        delay.delay_ms(25);

        // Set pixel format and MADCTL
        self.send_command_with_data(commands::MADCTL, &[0x00])?;
        self.send_command_with_data(commands::COLMOD, &[color.colmod_value()])?;

        // Set gaps as per C code
        self.x_gap = 0x06;
        self.y_gap = 0;

        self.send_command(commands::DISPON)?;
        info!("Display initialization complete");
        Ok(())
    }

    pub fn paint_screen(
        &mut self,
        color: u16,
    ) -> Result<(), DriverError<IFACE::Error, RST::Error>> {
        const CHUNK_HEIGHT: u16 = 50;
        const BYTES_PER_PIXEL: usize = 2; // Rgb565
        const CHUNK_SIZE: usize = 466 * CHUNK_HEIGHT as usize * BYTES_PER_PIXEL;

        let mut buffer = alloc::vec::Vec::with_capacity(CHUNK_SIZE);
        buffer.resize(CHUNK_SIZE / 2, color);
        let buffer_bytes = unsafe {
            core::slice::from_raw_parts(buffer.as_ptr() as *const u8, CHUNK_SIZE)
        };

        for y in (0..self.config.height).step_by(CHUNK_HEIGHT as usize) {
            let height = if y + CHUNK_HEIGHT <= self.config.height {
                CHUNK_HEIGHT
            } else {
                self.config.height - y
            };

            let x_start = self.x_gap;
            let x_end = self.config.width + self.x_gap;
            let y_start = y + self.y_gap;
            let y_end = y + height + self.y_gap;

            self.send_command_with_data(
                commands::CASET,
                &[
                    (x_start >> 8) as u8,
                    (x_start & 0xFF) as u8,
                    ((x_end - 1) >> 8) as u8,
                    ((x_end - 1) & 0xFF) as u8,
                ],
            )?;

            self.send_command_with_data(
                commands::PASET,
                &[
                    (y_start >> 8) as u8,
                    (y_start & 0xFF) as u8,
                    ((y_end - 1) >> 8) as u8,
                    ((y_end - 1) & 0xFF) as u8,
                ],
            )?;

            let len = ((x_end - x_start) * (y_end - y_start)) as usize * BYTES_PER_PIXEL;
            let chunk = &buffer_bytes[0..len];
            self.interface.send_pixels(chunk).map_err(DriverError::InterfaceError)?;
        }

        Ok(())
    }

    fn send_command(&mut self, cmd: u8) -> Result<(), DriverError<IFACE::Error, RST::Error>> {
        self.interface
            .send_command(cmd)
            .map_err(DriverError::InterfaceError)
    }

    fn send_command_with_data(
        &mut self,
        cmd: u8,
        data: &[u8],
    ) -> Result<(), DriverError<IFACE::Error, RST::Error>> {
        self.interface
            .send_command_with_data(cmd, data)
            .map_err(DriverError::InterfaceError)?;
        Ok(())
    }

    pub fn set_brightness(
        &mut self,
        value: u8,
    ) -> Result<(), DriverError<IFACE::Error, RST::Error>> {
        info!("Setting brightness to 0x{:02x}", value);
        self.send_command_with_data(commands::WRDISBV, &[value])
    }
}