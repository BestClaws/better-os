use alloc::boxed::Box;
use core::iter::once;
use async_trait::async_trait;
use embedded_hal_async::delay::DelayNs;
use embedded_hal::digital::OutputPin;
use display_interface::{DataFormat::{U16BEIter, U8Iter}, AsyncWriteOnlyDataCommand, DisplayError};
use display_interface_spi::SPIInterface;
use embassy_embedded_hal::shared_bus::asynch::spi::{SpiDevice, SpiDeviceWithConfig};
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use esp_hal::{Async, spi::master::Spi};
use esp_hal::gpio::Output;
use crate::system::hal::display::{AsyncDisplay, Orientation};

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

impl Mode for Orientation {
    fn mode(&self) -> u8 {
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

pub enum ModeState {
    On,
    Off,
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

pub struct Ili9341Driver<DC: OutputPin, RESET: OutputPin> {
    interface: SPIInterface<SpiDeviceWithConfig<'static, CriticalSectionRawMutex, Spi<'static, Async>, Output<'static>>, DC>,
    reset: RESET,
    width: usize,
    height: usize,
    landscape: bool,
}

impl<DC: OutputPin, RESET: OutputPin> Ili9341Driver<DC, RESET> {
    pub fn new(
        spi: SpiDeviceWithConfig<'static, CriticalSectionRawMutex, Spi<'static, Async>, Output<'static>>,
        dc: DC,
        reset: RESET,
    ) -> Self {
        let interface = SPIInterface::new(spi, dc);
        Self {
            interface,
            reset,
            width: DisplaySize240x320::WIDTH, // Default size
            height: DisplaySize240x320::HEIGHT,
            landscape: false, // Default to Portrait
        }
    }

    async fn command(&mut self, cmd: Command, args: &[u8]) -> Result<(), DisplayError> {
        self.interface
            .send_commands(U8Iter(&mut once(cmd as u8)))
            .await?;
        self.interface
            .send_data(U8Iter(&mut args.iter().cloned()))
            .await
    }

    async fn write_iter<I: IntoIterator<Item = u16>>(&mut self, data: I) -> Result<(), DisplayError> {
        self.command(Command::MemoryWrite, &[]).await?;
        let mut iter = data.into_iter();
        self.interface.send_data(U16BEIter(&mut iter)).await
    }

    async fn set_window(&mut self, x0: u16, y0: u16, x1: u16, y1: u16) -> Result<(), DisplayError> {
        self.command(
            Command::ColumnAddressSet,
            &[
                (x0 >> 8) as u8, x0 as u8,
                (x1 >> 8) as u8, x1 as u8,
            ],
        ).await?;

        self.command(
            Command::PageAddressSet,
            &[
                (y0 >> 8) as u8, y0 as u8,
                (y1 >> 8) as u8, y1 as u8,
            ],
        ).await
    }

    pub async fn draw_raw_iter<I: IntoIterator<Item = u16>>(
        &mut self,
        x0: u16,
        y0: u16,
        x1: u16,
        y1: u16,
        data: I,
    ) -> Result<(), DisplayError> {
        self.set_window(x0, y0, x1, y1).await?;
        self.write_iter(data).await
    }

    pub async fn clear_screen(&mut self, color: u16) -> Result<(), DisplayError> {
        let color = core::iter::repeat_n(color, self.width * self.height);
        self.draw_raw_iter(0, 0, self.width as u16 - 1, self.height as u16 - 1, color).await
    }

    pub async fn set_orientation<MODE: Mode>(&mut self, mode: MODE) -> Result<(), DisplayError> {
        self.command(Command::MemoryAccessControl, &[mode.mode()]).await?;
        if self.landscape ^ mode.is_landscape() {
            core::mem::swap(&mut self.height, &mut self.width);
        }
        self.landscape = mode.is_landscape();
        Ok(())
    }

    pub async fn sleep_mode(&mut self, mode: ModeState) -> Result<(), DisplayError> {
        match mode {
            ModeState::On => self.command(Command::SleepModeOn, &[]).await,
            ModeState::Off => self.command(Command::SleepModeOff, &[]).await,
        }
    }

    pub async fn display_mode(&mut self, mode: ModeState) -> Result<(), DisplayError> {
        match mode {
            ModeState::On => self.command(Command::DisplayOn, &[]).await,
            ModeState::Off => self.command(Command::DisplayOff, &[]).await,
        }
    }

    pub async fn invert_mode(&mut self, mode: ModeState) -> Result<(), DisplayError> {
        match mode {
            ModeState::On => self.command(Command::InvertOn, &[]).await,
            ModeState::Off => self.command(Command::InvertOff, &[]).await,
        }
    }

    pub async fn brightness(&mut self, brightness: u8) -> Result<(), DisplayError> {
        self.command(Command::SetBrightness, &[brightness]).await
    }

    pub fn width(&self) -> usize {
        self.width
    }

    pub fn height(&self) -> usize {
        self.height
    }
}

#[async_trait(?Send)]
impl<DC: OutputPin, RESET: OutputPin> AsyncDisplay for Ili9341Driver<DC, RESET> {
    async fn init(&mut self) {
        let mut delay = embassy_time::Delay;
        self.reset.set_low().map_err(|_| DisplayError::RSError).expect("Failed to set reset low");
        delay.delay_ms(1).await;
        self.reset.set_high().map_err(|_| DisplayError::RSError).expect("Failed to set reset high");
        delay.delay_ms(5).await;

        self.command(Command::SoftwareReset, &[]).await.expect("Failed to send software reset");
        delay.delay_ms(120).await;

        self.command(Command::PixelFormatSet, &[0x55]).await.expect("Failed to set pixel format");
        self.sleep_mode(ModeState::Off).await.expect("Failed to disable sleep mode");
        delay.delay_ms(5).await;
        self.display_mode(ModeState::On).await.expect("Failed to enable display");
        self.set_orientation(Orientation::Portrait).await.expect("Failed to set orientation");
    }
    async fn draw(&mut self, buffer: &[u8]) {
        const IN_W: usize = 120;
        const IN_H: usize = 160;
        const OUT_W: usize = 240;
        const OUT_H: usize = 320;

        let pixels = (0..OUT_H).flat_map(move |y| {
            let src_y = y / 2;
            (0..OUT_W).map(move |x| {
                let src_x = x / 2;
                let idx = src_y * IN_W + src_x;
                let rgb332 = buffer.get(idx).copied().unwrap_or(0);
                rgb332_to_rgb565(rgb332)
            })
        });

        self.draw_raw_iter(0, 0, (OUT_W - 1) as u16, (OUT_H - 1) as u16, pixels)
            .await
            .expect("Failed to draw buffer");
    }
    






    async fn clear(&mut self, color: u16) {
        self.clear_screen(color).await.expect("Failed to clear screen");
    }

    async fn set_orientation(&mut self, orientation: Orientation) {
        self.set_orientation(orientation).await.expect("Failed to set orientation");
    }

    fn get_width(&self) -> usize {
        self.width()
    }

    fn get_height(&self) -> usize {
        self.height()
    }
}

fn rgb332_to_rgb565(c: u8) -> u16 {
    let r = (c >> 5) & 0b111;     // 3 bits
    let g = (c >> 2) & 0b111;     // 3 bits
    let b = c & 0b11;             // 2 bits

    let r5 = (r << 3) | (r >> 0);      // expand 3-bit to 5-bit
    let g6 = (g << 3) | (g >> 0);      // expand 3-bit to 6-bit
    let b5 = (b << 3) | (b << 1) | (b >> 1); // expand 2-bit to 5-bit

    ((r5 as u16) << 11) | ((g6 as u16) << 5) | (b5 as u16)
}

fn rgb888_to_rgb565(r: u8, g: u8, b: u8) -> u16 {
    let r5 = (r >> 3) as u16;
    let g6 = (g >> 2) as u16;
    let b5 = (b >> 3) as u16;

    (r5 << 11) | (g6 << 5) | b5
}
