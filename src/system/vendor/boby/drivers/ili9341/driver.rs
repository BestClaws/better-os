//! ILI9341 Display Driver - Async
//!
//! Adapted from a synchronous version to support `embedded-hal-async`
//! and async `display-interface` traits. Stripped embedded-graphics support.
//! Provides a unified driver for the ILI9341 display with orientation and dimension support.

#![no_std]

use alloc::boxed::Box;
use async_trait::async_trait;
use core::iter::once;
use display_interface::{AsyncWriteOnlyDataCommand, DataFormat::{U16BEIter, U8Iter}, DisplayError};
use display_interface_spi::SPIInterface;
use embedded_hal::digital::OutputPin;
use embedded_hal_async::delay::DelayNs;
use embassy_embedded_hal::shared_bus::asynch::spi::SpiDevice;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use esp_hal::{spi::master::Spi, Async};
use crate::system::hal::display::AsyncDisplay;

pub type Result<T = (), E = DisplayError> = core::result::Result<T, E>;

pub trait DisplaySize {
    const WIDTH: usize;
    const HEIGHT: usize;
}

pub struct DisplaySize240x320;
impl DisplaySize for DisplaySize240x320 {
    const WIDTH: usize = 240;
    const HEIGHT: usize = 320;
}

pub struct DisplaySize320x480;
impl DisplaySize for DisplaySize320x480 {
    const WIDTH: usize = 320;
    const HEIGHT: usize = 480;
}

pub trait Mode {
    fn mode(&self) -> u8;
    fn is_landscape(&self) -> bool;
}

pub enum Orientation {
    Portrait,
    PortraitFlipped,
    Landscape,
    LandscapeFlipped,
}

impl Mode for Orientation {
    fn mode(&self) -> u8 {
        match self {
            Self::Portrait => 0x40 | 0x08, // MY=1, MX=0, MV=0
            Self::PortraitFlipped => 0x80 | 0x08, // MY=0, MX=0, MV=0
            Self::Landscape => 0x20 | 0x08, // MY=0, MX=1, MV=1
            Self::LandscapeFlipped => 0x40 | 0x80 | 0x20 | 0x08, // MY=1, MX=0, MV=1
        }
    }

    fn is_landscape(&self) -> bool {
        matches!(self, Self::Landscape | Self::LandscapeFlipped)
    }
}

pub enum ModeState {
    On,
    Off,
}

pub struct Ili9341Async<IFACE, RESET> {
    interface: IFACE,
    reset: RESET,
    width: usize,
    height: usize,
    landscape: bool,
    orientation: Orientation,
}

impl<IFACE, RESET> Ili9341Async<IFACE, RESET>
where
    IFACE: AsyncWriteOnlyDataCommand,
    RESET: OutputPin,
{
    pub fn new_instance<DELAY, SIZE, MODE>(
        interface: IFACE,
        reset: RESET,
        delay: &mut DELAY,
        mode: MODE,
        display_size: SIZE,
    ) -> Self
    where
        DELAY: DelayNs,
        SIZE: DisplaySize,
        MODE: Mode,
    {
        let mut display = Self {
            interface,
            reset,
            width: SIZE::WIDTH,
            height: SIZE::HEIGHT,
            landscape: mode.is_landscape(),
            orientation: match mode.mode() {
                0x48 => Orientation::Portrait,
                0x88 => Orientation::PortraitFlipped,
                0x28 => Orientation::Landscape,
                0xE8 => Orientation::LandscapeFlipped,
                _ => Orientation::Portrait, // Fallback
            },
        };
        if mode.is_landscape() {
            core::mem::swap(&mut display.width, &mut display.height);
        }
        display
    }

    pub async fn init_display<DELAY, MODE>(&mut self, delay: &mut DELAY, mode: MODE) -> Result
    where
        DELAY: DelayNs,
        MODE: Mode,
    {
        self.reset.set_low().map_err(|_| DisplayError::RSError)?;
        delay.delay_ms(10).await;
        self.reset.set_high().map_err(|_| DisplayError::RSError)?;
        delay.delay_ms(10).await;

        self.command(Command::SoftwareReset, &[]).await?;
        delay.delay_ms(150).await;

        self.set_orientation(mode).await?;
        self.command(Command::PixelFormatSet, &[0x55]).await?;
        self.sleep_mode(ModeState::Off).await?;
        delay.delay_ms(10).await;
        self.display_mode(ModeState::On).await?;

        Ok(())
    }

    async fn command(&mut self, cmd: Command, args: &[u8]) -> Result {
        self.interface
            .send_commands(U8Iter(&mut once(cmd as u8)))
            .await?;
        self.interface
            .send_data(U8Iter(&mut args.iter().cloned()))
            .await
    }

    async fn write_iter<I: IntoIterator<Item = u16>>(&mut self, data: I) -> Result {
        self.command(Command::MemoryWrite, &[]).await?;
        let mut iter = data.into_iter();
        self.interface.send_data(U16BEIter(&mut iter)).await
    }

    async fn set_window(&mut self, x0: u16, y0: u16, x1: u16, y1: u16) -> Result {
        if x1 >= self.width as u16 || y1 >= self.height as u16 {
            return Err(DisplayError::OutOfBoundsError);
        }

        let (col_start, col_start_low, col_end, col_end_low, page_start, page_start_low, page_end, page_end_low) =
            if self.landscape {
                // Landscape and LandscapeFlipped: swap x and y (MV=1)
                (
                    (y0 >> 8) as u8, y0 as u8, (y1 >> 8) as u8, y1 as u8, // Columns = y
                    (x0 >> 8) as u8, x0 as u8, (x1 >> 8) as u8, x1 as u8, // Pages = x
                )
            } else {
                // Portrait and PortraitFlipped: normal mapping
                (
                    (x0 >> 8) as u8, x0 as u8, (x1 >> 8) as u8, x1 as u8, // Columns = x
                    (y0 >> 8) as u8, y0 as u8, (y1 >> 8) as u8, y1 as u8, // Pages = y
                )
            };

        self.command(
            Command::ColumnAddressSet,
            &[col_start, col_start_low, col_end, col_end_low],
        ).await?;
        self.command(
            Command::PageAddressSet,
            &[page_start, page_start_low, page_end, page_end_low],
        ).await
    }

    pub async fn draw_raw_iter<I: IntoIterator<Item = u16>>(
        &mut self,
        x0: u16,
        y0: u16,
        x1: u16,
        y1: u16,
        data: I,
    ) -> Result {
        self.set_window(x0, y0, x1, y1).await?;
        self.write_iter(data).await
    }

    pub async fn clear_screen(&mut self, color: u16) -> Result {
        if self.width == 0 || self.height == 0 {
            return Err(DisplayError::RSError);
        }
        let color = core::iter::repeat_n(color, self.width * self.height);
        self.draw_raw_iter(0, 0, self.width as u16 - 1, self.height as u16 - 1, color).await
    }

    pub async fn set_orientation<MODE: Mode>(&mut self, mode: MODE) -> Result {
        self.command(Command::MemoryAccessControl, &[mode.mode()]).await?;
        if self.landscape ^ mode.is_landscape() {
            core::mem::swap(&mut self.height, &mut self.width);
        }
        self.landscape = mode.is_landscape();
        self.orientation = match mode.mode() {
            0x48 => Orientation::Portrait,
            0x88 => Orientation::PortraitFlipped,
            0x28 => Orientation::Landscape,
            0xE8 => Orientation::LandscapeFlipped,
            _ => Orientation::Portrait, // Fallback
        };
        Ok(())
    }

    pub async fn sleep_mode(&mut self, mode: ModeState) -> Result {
        match mode {
            ModeState::On => self.command(Command::SleepModeOn, &[]).await,
            ModeState::Off => self.command(Command::SleepModeOff, &[]).await,
        }
    }

    pub async fn display_mode(&mut self, mode: ModeState) -> Result {
        match mode {
            ModeState::On => self.command(Command::DisplayOn, &[]).await,
            ModeState::Off => self.command(Command::DisplayOff, &[]).await,
        }
    }

    pub async fn invert_mode(&mut self, mode: ModeState) -> Result {
        match mode {
            ModeState::On => self.command(Command::InvertOn, &[]).await,
            ModeState::Off => self.command(Command::InvertOff, &[]).await,
        }
    }

    pub async fn brightness(&mut self, brightness: u8) -> Result {
        self.command(Command::SetBrightness, &[brightness]).await
    }

    pub fn width(&self) -> usize {
        self.width
    }

    pub fn height(&self) -> usize {
        self.height
    }
}

#[derive(Clone, Copy)]
enum Command {
    SoftwareReset = 0x01,
    MemoryAccessControl = 0x36,
    PixelFormatSet = 0x3a,
    SleepModeOn = 0x10,
    SleepModeOff = 0x11,
    InvertOff = 0x20,
    InvertOn = 0x21,
    DisplayOff = 0x28,
    DisplayOn = 0x29,
    ColumnAddressSet = 0x2a,
    PageAddressSet = 0x2b,
    MemoryWrite = 0x2c,
    SetBrightness = 0x51,
}

pub struct Ili9341Driver<CS: OutputPin, DC: OutputPin, RESET: OutputPin> {
    display: Ili9341Async<SPIInterface<SpiDevice<'static, CriticalSectionRawMutex, Spi<'static, Async>, CS>, DC>, RESET>,
}

impl<CS: OutputPin, DC: OutputPin, RESET: OutputPin> Ili9341Driver<CS, DC, RESET> {
    pub fn init(
        spi: SpiDevice<'static, CriticalSectionRawMutex, Spi<'static, Async>, CS>,
        dc: DC,
        reset: RESET,
    ) -> Self {
        let interface = SPIInterface::new(spi, dc);
        let mut delay = embassy_time::Delay;

        let display = Ili9341Async::new_instance(
            interface,
            reset,
            &mut delay,
            Orientation::Landscape,
            DisplaySize240x320,
        );

        Self { display }
    }
}

#[async_trait(?Send)]
impl<CS: OutputPin, DC: OutputPin, RESET: OutputPin> AsyncDisplay for Ili9341Driver<CS, DC, RESET> {
    async fn init(&mut self) {
        let mut delay = embassy_time::Delay;
        self.display
            .init_display(&mut delay, Orientation::Landscape)
            .await
            .expect("Display initialization failed");
    }

    async fn draw(&mut self, buffer: &[u8]) {
        // Placeholder: Convert buffer to u16 if needed
        self.display.clear_screen(0xF800).await.unwrap(); // Use red for testing
    }

    async fn clear(&mut self, color: u16) {
        self.display.clear_screen(color).await.unwrap();
    }

    async fn set_orientation(&mut self, orientation: Orientation) {
        self.display.set_orientation(orientation).await.unwrap();
    }

    fn width(&self) -> usize {
        self.display.width()
    }

    fn height(&self) -> usize {
        self.display.height()
    }
}
