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

/// Computes the framebuffer size (in bytes) for a given display and color mode.
pub const fn framebuffer_size(display: DisplaySize, color: ColorMode) -> usize {
    (display.width as usize) * (display.height as usize) * color.bytes_per_pixel()
}

/// Frambuffer enum to hold either a static array or a boxed array
pub enum Framebuffer {
    Static(&'static mut [u8]),
    Heap(Box<[u8]>),
}

impl Framebuffer {
    pub fn as_mut_slice(&mut self) -> &mut [u8] {
        match self {
            Framebuffer::Static(ref mut arr) => arr,
            Framebuffer::Heap(ref mut boxed) => boxed,
        }
    }

    pub fn as_slice(&self) -> &[u8] {
        match self {
            Framebuffer::Static(ref arr) => arr,
            Framebuffer::Heap(ref boxed) => boxed,
        }
    }

    pub fn len(&self) -> usize {
        self.as_slice().len()
    }
}

impl core::ops::Deref for Framebuffer {
    type Target = [u8];
    fn deref(&self) -> &Self::Target {
        match self {
            Framebuffer::Static(arr) => arr,
            Framebuffer::Heap(boxed) => boxed,
        }
    }
}

impl core::ops::DerefMut for Framebuffer {
    fn deref_mut(&mut self) -> &mut Self::Target {
        match self {
            Framebuffer::Static(arr) => arr,
            Framebuffer::Heap(boxed) => boxed,
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
    pub(crate) framebuffer: Framebuffer,
    pub(crate) config: DisplaySize,
    x_gap: u16,
    y_gap: u16,
}

impl<IFACE, RST> Sh8601Driver<IFACE, RST>
where
    IFACE: ControllerInterface,
    RST: ResetInterface,
{
    pub fn new_static<DELAY, const N: usize>(
        interface: IFACE,
        reset: RST,
        color: ColorMode,
        config: DisplaySize,
        mut delay: DELAY,
        framebuffer: &'static mut [u8; N],
    ) -> Result<Self, DriverError<IFACE::Error, RST::Error>>
    where
        DELAY: DelayNs,
    {
        let mut driver = Self {
            interface,
            reset,
            framebuffer: Framebuffer::Static(&mut framebuffer[..]),
            config,
            x_gap: 0,
            y_gap: 0,
        };
        driver.hard_reset()?;
        driver.initialize_display(&mut delay, color)?;
        Ok(driver)
    }

    pub fn new_heap<DELAY, const N: usize>(
        interface: IFACE,
        reset: RST,
        color: ColorMode,
        config: DisplaySize,
        mut delay: DELAY,
    ) -> Result<Self, DriverError<IFACE::Error, RST::Error>>
    where
        DELAY: DelayNs,
    {
        let mut driver = Self {
            interface,
            reset,
            framebuffer: Framebuffer::Heap(Box::new([0u8; N])),
            config,
            x_gap: 0,
            y_gap: 0,
        };
        driver.hard_reset()?;
        driver.initialize_display(&mut delay, color)?;
        Ok(driver)
    }

    pub fn hard_reset(&mut self) -> Result<(), DriverError<IFACE::Error, RST::Error>> {
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
        Ok(())
    }

    pub fn paint_screen(
        &mut self,
        color: u16,
    ) -> Result<(), DriverError<IFACE::Error, RST::Error>> {
        const CHUNK_HEIGHT: u16 = 50;
        const BYTES_PER_PIXEL: usize = 2; // RGB565
        const CHUNK_SIZE: usize = 466 * CHUNK_HEIGHT as usize * BYTES_PER_PIXEL;

        let mut buffer = alloc::vec::Vec::with_capacity(CHUNK_SIZE);
        buffer.resize(CHUNK_SIZE, 0);
        for i in 0..(466 * CHUNK_HEIGHT as usize) {
            buffer[i * 2] = (color >> 8) as u8;
            buffer[i * 2 + 1] = (color & 0xFF) as u8;
        }

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
            let chunk = &buffer[0..len];
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

    pub fn sleep_in<DELAY>(
        &mut self,
        delay: &mut DELAY,
    ) -> Result<(), DriverError<IFACE::Error, RST::Error>>
    where
        DELAY: DelayNs,
    {
        self.send_command(commands::SLPIN)?;
        delay.delay_ms(5);
        Ok(())
    }

    pub fn sleep_out<DELAY>(
        &mut self,
        delay: &mut DELAY,
    ) -> Result<(), DriverError<IFACE::Error, RST::Error>>
    where
        DELAY: DelayNs,
    {
        self.send_command(commands::SLPOUT)?;
        delay.delay_ms(5);
        Ok(())
    }

    pub fn display_off(&mut self) -> Result<(), DriverError<IFACE::Error, RST::Error>> {
        self.send_command(commands::DISPOFF)
    }

    pub fn display_on(&mut self) -> Result<(), DriverError<IFACE::Error, RST::Error>> {
        self.send_command(commands::DISPON)
    }

    pub fn set_window(
        &mut self,
        x_start: u16,
        y_start: u16,
        x_end: u16,
        y_end: u16,
    ) -> Result<(), DriverError<IFACE::Error, RST::Error>> {
        if x_end == 0 || y_end == 0 {
            return Err(DriverError::InvalidConfiguration(
                "Window width/height cannot be zero",
            ));
        }
        if x_start >= self.config.width || y_start >= self.config.height {
            return Err(DriverError::InvalidConfiguration(
                "Window start coordinates out of bounds",
            ));
        }

        if x_end < x_start || y_end < y_start {
            return Err(DriverError::InvalidConfiguration(
                "Invalid window dimensions (end < start)",
            ));
        }

        self.send_command_with_data(
            commands::CASET,
            &[
                (x_start >> 8) as u8,
                (x_start & 0xFF) as u8,
                (x_end >> 8) as u8,
                (x_end & 0xFF) as u8,
            ],
        )?;

        self.send_command_with_data(
            commands::PASET,
            &[
                (y_start >> 8) as u8,
                (y_start & 0xFF) as u8,
                (y_end >> 8) as u8,
                (y_end & 0xFF) as u8,
            ],
        )?;
        Ok(())
    }

    pub fn set_madctl(&mut self, value: u8) -> Result<(), DriverError<IFACE::Error, RST::Error>> {
        self.send_command_with_data(commands::MADCTL, &[value])
    }

    pub fn flush(&mut self) -> Result<(), DriverError<IFACE::Error, RST::Error>> {
        self.set_window(0, 0, self.config.width - 1, self.config.height - 1)?;
        self.interface
            .send_pixels(&self.framebuffer)
            .map_err(DriverError::InterfaceError)?;
        Ok(())
    }

    pub fn partial_flush(
        &mut self,
        x_start: u16,
        x_end: u16,
        y_start: u16,
        y_end: u16,
        color: ColorMode,
    ) -> Result<(), DriverError<IFACE::Error, RST::Error>> {
        self.set_window(x_start, y_start, x_end, y_end)?;
        let bytes_per_pixel = color.bytes_per_pixel();
        let fb_width = self.config.width as usize * bytes_per_pixel;
        let width = (x_end - x_start + 1) as usize;
        let height = (y_end - y_start + 1) as usize;
        let mut pixel_data = alloc::vec::Vec::with_capacity(width * height * bytes_per_pixel);

        for y in 0..height {
            let offset = (y_start as usize + y) * fb_width + (x_start as usize * bytes_per_pixel);
            let row_end = offset + (width * bytes_per_pixel);
            if offset < self.framebuffer.len() && row_end <= self.framebuffer.len() {
                pixel_data.extend_from_slice(&self.framebuffer[offset..row_end]);
            } else {
                return Err(DriverError::InvalidConfiguration(
                    "Framebuffer slice out of bounds",
                ));
            }
        }

        self.interface
            .send_pixels(&pixel_data)
            .map_err(DriverError::InterfaceError)?;
        Ok(())
    }
}