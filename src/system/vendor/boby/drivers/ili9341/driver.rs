use alloc::boxed::Box;
use alloc::vec;
use alloc::vec::Vec;
use core::iter::once;
use async_trait::async_trait;
use defmt::info;
use embedded_hal_async::delay::DelayNs;
use embedded_hal::digital::OutputPin;
use display_interface::{DataFormat::{U16BEIter, U8Iter, U16BE}, AsyncWriteOnlyDataCommand, DisplayError, DataFormat};
use display_interface_spi::SPIInterface;
use embedded_hal_async::spi::SpiDevice;
use crate::system::hal::display::{AsyncDisplay, Orientation};
use crate::system::kernel::config::resources::{FRAME_BUFFER_HEIGHT, FRAME_BUFFER_WIDTH};

// Precomputed LUT for 4-bit grayscale to RGB565 (linear mapping, no gamma correction)
const GRAY4_LUT: [u16; 16] = {
    let mut lut = [0u16; 16];
    let mut gray = 0;
    while gray < 16 {
        // Map 0-15 to 0-31 (5-bit) for R/B, 0-63 (6-bit) for G
        let intensity = (gray * 255 / 15) as u16; // Linear scaling to 8-bit
        let r = (intensity >> 3) & 0x1F; // 5-bit red
        let g = (intensity >> 2) & 0x3F; // 6-bit green
        let b = (intensity >> 3) & 0x1F; // 5-bit blue
        lut[gray as usize] = (r << 11) | (g << 5) | b;
        gray += 1;
    }
    lut
};

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
    PositiveGammaCorrection = 0xE0,
    NegativeGammaCorrection = 0xE1,
}

pub struct Ili9341Driver<SPI, DC: OutputPin, RESET: OutputPin> {
    interface: SPIInterface<SPI, DC>,
    reset: RESET,
    width: u32,
    height: u32,
    landscape: bool,
    line_buf: Vec<u16>, // Reusable buffer for drawing
}

impl<SPI: SpiDevice, DC: OutputPin, RESET: OutputPin> Ili9341Driver<SPI, DC, RESET> {
    pub fn new(spi: SPI, dc: DC, reset: RESET) -> Self {
        let interface = SPIInterface::new(spi, dc);
        Self {
            interface,
            reset,
            width: 240,
            height: 320,
            landscape: false,
            line_buf: vec![0u16; (FRAME_BUFFER_WIDTH * 4) as usize], // Max scale = 4
        }
    }

    async fn command(&mut self, cmd: Command) {
        self.interface
            .send_commands(U8Iter(&mut once(cmd as u8)))
            .await
            .unwrap();
    }

    async fn command_with_args(&mut self, cmd: Command, args: &[u8]) {
        self.interface
            .send_commands(U8Iter(&mut once(cmd as u8)))
            .await
            .unwrap();
        self.interface
            .send_data(DataFormat::U8(args))
            .await
            .unwrap();
    }

    async fn set_window(&mut self, x0: u16, y0: u16, x1: u16, y1: u16) {
        self.command_with_args(
            Command::ColumnAddressSet,
            &[
                (x0 >> 8) as u8, x0 as u8,
                (x1 >> 8) as u8, x1 as u8,
            ],
        ).await;

        self.command_with_args(
            Command::PageAddressSet,
            &[
                (y0 >> 8) as u8, y0 as u8,
                (y1 >> 8) as u8, y1 as u8,
            ],
        ).await;
    }

    async fn write_iter<I: IntoIterator<Item = u16>>(&mut self, data: I) {
        self.command(Command::MemoryWrite).await;
        let mut iter = data.into_iter();
        self.interface.send_data(U16BEIter(&mut iter)).await.unwrap();
    }

    pub async fn draw_raw_iter<I: IntoIterator<Item = u16>>(
        &mut self,
        x0: u16,
        y0: u16,
        x1: u16,
        y1: u16,
        data: I,
    ) {
        self.set_window(x0, y0, x1, y1).await;
        self.write_iter(data).await;
    }

    pub async fn clear_screen(&mut self, color: u16) {
        let color = core::iter::repeat_n(color, (self.width * self.height) as usize);
        self.draw_raw_iter(0, 0, self.width as u16 - 1, self.height as u16 - 1, color).await;
    }

    pub async fn set_orientation(&mut self, orientation: Orientation) {
        self.command_with_args(Command::MemoryAccessControl, &[orientation.display_mode()]).await;
        if self.landscape ^ orientation.is_landscape() {
            core::mem::swap(&mut self.height, &mut self.width);
        }
        self.landscape = orientation.is_landscape();
    }

    pub async fn sleep_mode(&mut self, sleep: bool) {
        if sleep {
            self.command(Command::SleepModeOn).await;
        } else {
            self.command(Command::SleepModeOff).await;
        }
    }

    pub async fn display_power_mode(&mut self, on: bool) {
        if on {
            self.command(Command::DisplayOn).await;
        } else {
            self.command(Command::DisplayOff).await;
        }
    }

    pub async fn invert_mode(&mut self, invert: bool) {
        if invert {
            self.command(Command::InvertOn).await;
        } else {
            self.command(Command::InvertOff).await;
        }
    }

    pub async fn brightness(&mut self, brightness: u8) {
        self.command_with_args(Command::SetBrightness, &[brightness]).await;
    }

    pub fn width(&self) -> u32 {
        self.width
    }

    pub fn height(&self) -> u32 {
        self.height
    }

    async fn write_slice(&mut self, data: &[u16]) {
        self.command(Command::MemoryWrite).await;
        self.interface.send_data(DataFormat::U16(data)).await.unwrap();
    }

    async fn draw_raw_slice(&mut self, x0: u16, y0: u16, x1: u16, y1: u16, data: &[u16]) {
        self.set_window(x0, y0, x1, y1).await;
        self.write_slice(data).await;
    }
}

#[async_trait(?Send)]
impl<SPI: SpiDevice, DC: OutputPin, RESET: OutputPin> AsyncDisplay for Ili9341Driver<SPI, DC, RESET> {
    async fn init(&mut self) {
        let mut delay = embassy_time::Delay;
        self.reset.set_low().unwrap();
        delay.delay_ms(1).await;
        self.reset.set_high().unwrap();
        delay.delay_ms(5).await;

        self.command(Command::SoftwareReset).await;
        delay.delay_ms(120).await;

        self.command_with_args(Command::PixelFormatSet, &[0x55]).await;
        self.command_with_args(
            Command::PositiveGammaCorrection,
            &[
                0x0F, 0x31, 0x2B, 0x0C, 0x0E, 0x08, 0x4E, 0xF1,
                0x37, 0x07, 0x10, 0x03, 0x0E, 0x09, 0x00,
            ],
        ).await;
        self.command_with_args(
            Command::NegativeGammaCorrection,
            &[
                0x00, 0x0E, 0x14, 0x03, 0x11, 0x07, 0x31, 0xC1,
                0x48, 0x08, 0x0F, 0x0C, 0x31, 0x36, 0x0F,
            ],
        ).await;

        self.sleep_mode(false).await;
        delay.delay_ms(5).await;
        self.display_power_mode(true).await;
        self.set_orientation(Orientation::LandscapeFlipped).await;
    }

    async fn draw_gray4(&mut self, buffer: &[u8], scale: u32) {
        let out_w = FRAME_BUFFER_WIDTH * scale;
        let out_h = FRAME_BUFFER_HEIGHT * scale;

        // Validate buffer size and scale
        let expected_buffer_size = ((FRAME_BUFFER_WIDTH * FRAME_BUFFER_HEIGHT + 1) / 2) as usize;
        if buffer.len() < expected_buffer_size || ![1, 2, 4].contains(&scale) {
            info!("Invalid input: buffer len {}, scale {}", buffer.len(), scale);
            return;
        }

        let batch_size = scale as usize; // Send 'scale' lines at once
        let total_buf_size = out_w as usize * batch_size;
        if self.line_buf.len() < total_buf_size {
            self.line_buf.resize(total_buf_size, 0u16);
        }

        self.set_window(0, 0, (out_w - 1) as u16, (out_h - 1) as u16).await;
        self.command(Command::MemoryWrite).await;

        for src_y in 0..FRAME_BUFFER_HEIGHT {
            for dy in 0..scale {
                let line_idx = dy * out_w;
                let mut pixel_idx = line_idx;
                // Process source pixels in pairs (two per byte)
                for src_x in (0..FRAME_BUFFER_WIDTH).step_by(2) {
                    let idx = (src_y * FRAME_BUFFER_WIDTH + src_x) as usize / 2;
                    let byte = buffer[idx];
                    let nibble1 = byte >> 4;
                    let nibble2 = byte & 0x0F;
                    let rgb565_1 = GRAY4_LUT[nibble1 as usize];
                    let rgb565_2 = GRAY4_LUT[nibble2 as usize];

                    for _ in 0..scale {
                        self.line_buf[pixel_idx as usize] = rgb565_1;
                        pixel_idx += 1;
                        if src_x + 1 < FRAME_BUFFER_WIDTH {
                            self.line_buf[pixel_idx as usize] = rgb565_2;
                            pixel_idx += 1;
                        }
                    }
                }
            }
            // Send batched lines
            self.interface.send_data(U16BE(&mut self.line_buf[..out_w as usize * batch_size])).await.unwrap();
        }
    }

    async fn draw(&mut self, buffer: &[u8], scale: u32) {
        let out_w = FRAME_BUFFER_WIDTH * scale;
        let out_h = FRAME_BUFFER_HEIGHT * scale;

        let expected_buffer_size = (FRAME_BUFFER_WIDTH * FRAME_BUFFER_HEIGHT * 2) as usize;
        if buffer.len() < expected_buffer_size || ![1, 2, 4].contains(&scale) {
            info!("Invalid input: buffer len {}, scale {}", buffer.len(), scale);
            return;
        }

        if self.line_buf.len() < out_w as usize {
            self.line_buf.resize(out_w as usize, 0u16);
        }

        self.set_window(0, 0, (out_w - 1) as u16, (out_h - 1) as u16).await;
        self.command(Command::MemoryWrite).await;

        for src_y in 0..FRAME_BUFFER_HEIGHT {
            for _ in 0..scale {
                let mut pixel_idx = 0;
                for src_x in 0..FRAME_BUFFER_WIDTH {
                    let idx = (src_y * FRAME_BUFFER_WIDTH + src_x) as usize * 2;
                    let rgb565 = ((buffer[idx] as u16) << 8) | (buffer[idx + 1] as u16);
                    for _ in 0..scale {
                        self.line_buf[pixel_idx] = rgb565;
                        pixel_idx += 1;
                    }
                }
                self.interface.send_data(DataFormat::U16BE(&mut self.line_buf[..out_w as usize])).await.unwrap();
            }
        }
    }

    async fn clear(&mut self, color: u16) {
        self.clear_screen(color).await;
    }

    async fn set_orientation(&mut self, orientation: Orientation) {
        self.set_orientation(orientation).await;
    }

    fn get_width(&self) -> u32 {
        self.width()
    }

    fn get_height(&self) -> u32 {
        self.height()
    }
}