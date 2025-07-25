use alloc::boxed::Box;
use alloc::vec;
use core::iter::once;
use async_trait::async_trait;
use defmt::info;
use embedded_hal_async::delay::DelayNs;
use embedded_hal::digital::OutputPin;
use display_interface::{DataFormat::{U16BEIter, U8Iter}, AsyncWriteOnlyDataCommand, DisplayError, DataFormat};
use display_interface_spi::SPIInterface;
use embedded_hal_async::spi::SpiDevice;
use crate::system::hal::display::{AsyncDisplay, Orientation};
use crate::system::kernel::config::resources::{FRAME_BUFFER_HEIGHT, FRAME_BUFFER_WIDTH};






impl  Orientation {
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

pub struct Ili9341Driver<SPI, DC: OutputPin, RESET: OutputPin> {
    interface: SPIInterface<SPI, DC>,
    reset: RESET,
    width: u32,
    height: u32,
    landscape: bool,
}

impl<SPI: SpiDevice, DC: OutputPin, RESET: OutputPin> Ili9341Driver<SPI, DC, RESET> {
    pub fn new(
        spi: SPI,
        dc: DC,
        reset: RESET,
    ) -> Self {
        let interface = SPIInterface::new(spi, dc);
        Self {
            interface,
            reset,
            width: 240,
            height: 320,
            landscape: false,
        }
    }

    async fn command(&mut self, cmd: Command) -> Result<(), DisplayError> {
        self.interface
            .send_commands(U8Iter(&mut once(cmd as u8)))
            .await
    }


    async fn command_with_args(&mut self, cmd: Command, args: &[u8]) -> Result<(), DisplayError> {
        self.interface
            .send_commands(U8Iter(&mut once(cmd as u8)))
            .await?;
        self.interface
            .send_data(DataFormat::U8(args))
            .await
    }

    async fn write_iter<I: IntoIterator<Item = u16>>(&mut self, data: I) -> Result<(), DisplayError> {
        self.command(Command::MemoryWrite).await?;
        let mut iter = data.into_iter();
        self.interface.send_data(U16BEIter(&mut iter)).await
    }

    async fn set_window(&mut self, x0: u16, y0: u16, x1: u16, y1: u16) -> Result<(), DisplayError> {
        self.command_with_args(
            Command::ColumnAddressSet,
            &[
                (x0 >> 8) as u8, x0 as u8,
                (x1 >> 8) as u8, x1 as u8,
            ],
        ).await?;

        self.command_with_args(
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
        let color = core::iter::repeat_n(color, (self.width * self.height) as usize);
        self.draw_raw_iter(0, 0, self.width as u16 - 1, self.height as u16 - 1, color).await
    }

    pub async fn set_orientation(&mut self, orientation: Orientation) -> Result<(), DisplayError> {
        self.command_with_args(Command::MemoryAccessControl, &[orientation.display_mode()]).await?;
        if self.landscape ^ orientation.is_landscape() {
            core::mem::swap(&mut self.height, &mut self.width);
        }
        self.landscape = orientation.is_landscape();
        Ok(())
    }

    pub async fn sleep_mode(&mut self, sleep: bool) -> Result<(), DisplayError> {
        if sleep {
            self.command(Command::SleepModeOn).await
        } else {
            self.command(Command::SleepModeOff).await
        }
    }

    pub async fn display_power_mode(&mut self, on: bool) -> Result<(), DisplayError> {
        if on {
            self.command(Command::DisplayOn).await
        } else {
            self.command(Command::DisplayOff).await
        }
    }

    pub async fn invert_mode(&mut self, invert: bool) -> Result<(), DisplayError> {
        if invert {
            self.command(Command::InvertOn).await
        } else {
            self.command(Command::InvertOff).await
        }
    }

    pub async fn brightness(&mut self, brightness: u8) -> Result<(), DisplayError> {
        self.command_with_args(Command::SetBrightness, &[brightness]).await
    }

    pub fn width(&self) -> u32 {
        self.width
    }

    pub fn height(&self) -> u32 {
        self.height
    }

    async fn write_slice(&mut self, data: &[u16]) {
        self.command(Command::MemoryWrite).await.unwrap();
        self.interface.send_data(DataFormat::U16(data)).await.unwrap();
    }

    async fn draw_raw_slice(&mut self, x0: u16, y0: u16, x1: u16, y1: u16, data: &[u16]) {
        self.set_window(x0, y0, x1, y1).await.unwrap();
        self.write_slice(data).await;
    }
}

#[async_trait(?Send)]
impl<SPI: SpiDevice, DC: OutputPin, RESET: OutputPin> AsyncDisplay for Ili9341Driver<SPI, DC, RESET> {
    async fn init(&mut self) {
        let mut delay = embassy_time::Delay;
        self.reset.set_low().map_err(|_| DisplayError::RSError).expect("Failed to set reset low");
        delay.delay_ms(1).await;
        self.reset.set_high().map_err(|_| DisplayError::RSError).expect("Failed to set reset high");
        delay.delay_ms(5).await;

        self.command(Command::SoftwareReset).await.expect("Failed to send software reset");
        delay.delay_ms(120).await;

        self.command_with_args(Command::PixelFormatSet, &[0x55]).await.expect("Failed to set pixel format");
        self.sleep_mode(false).await.expect("Failed to disable sleep mode");
        delay.delay_ms(5).await;
        self.display_power_mode(true).await.expect("Failed to enable display");
        self.set_orientation(Orientation::LandscapeFlipped).await.expect("Failed to set orientation");
    }

    async fn draw(&mut self, buffer: &[u8], scale: u32) {
        let out_w = FRAME_BUFFER_WIDTH * scale;
        let out_h = FRAME_BUFFER_HEIGHT * scale;

        // Verify buffer size
        let expected_buffer_size = (FRAME_BUFFER_WIDTH * FRAME_BUFFER_HEIGHT * 2) as usize;
        if buffer.len() < expected_buffer_size {
            info!("Buffer too small: got {} bytes, expected {}", buffer.len(), expected_buffer_size);
            return;
        }

        let mut line_buf = vec![0u16; out_w as usize];
        self.set_window(0, 0, (out_w - 1) as u16, (out_h - 1) as u16).await.unwrap();
        self.command(Command::MemoryWrite).await.unwrap();

        for y in 0..out_h {
            let src_y = y / scale;
            if src_y >= FRAME_BUFFER_HEIGHT {
                info!("Invalid src_y: {}", src_y);
                continue;
            }
            for x in 0..out_w {
                let src_x = x / scale;
                if src_x >= FRAME_BUFFER_WIDTH {
                    info!("Invalid src_x: {}", src_x);
                    continue;
                }
                let idx = (src_y * FRAME_BUFFER_WIDTH + src_x) as usize * 2;
                let rgb565 = if idx + 1 < buffer.len() {
                    ((buffer[idx] as u16) << 8) | (buffer[idx + 1] as u16) // Big-endian RGB565
                } else {
                    info!("Buffer index out of bounds: idx {}", idx);
                    0 // Default to black
                };
                line_buf[x as usize] = rgb565;
            }
            self.interface.send_data(DataFormat::U16BE(&mut line_buf)).await.unwrap();
        }
    }
    async fn clear(&mut self, color: u16) {
        self.clear_screen(color).await.expect("Failed to clear screen");
    }

    async fn set_orientation(&mut self, orientation: Orientation) {
        self.set_orientation(orientation).await.expect("Failed to set orientation");
    }

    fn get_width(&self) -> u32 {
        self.width()
    }

    fn get_height(&self) -> u32 {
        self.height()
    }
}

