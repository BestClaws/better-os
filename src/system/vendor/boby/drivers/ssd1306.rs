use alloc::boxed::Box;
use async_trait::async_trait;
use embassy_embedded_hal::shared_bus::asynch::i2c::I2cDevice;
use embassy_sync::blocking_mutex::raw::{CriticalSectionRawMutex, };
use esp_hal::Async;
use esp_hal::i2c::master::I2c;
use ssd1306::{I2CDisplayInterface, Ssd1306Async};
use ssd1306::mode::BufferedGraphicsModeAsync;
use ssd1306::prelude::*;
use crate::system::hal::display::AsyncDisplay;
use embedded_graphics::{
    image::Image,
    pixelcolor::BinaryColor,
    prelude::*,
};
use tinybmp::Bmp;

const BOOT1: &[u8] = include_bytes!("../../../../assets/boot_logo.bmp");

const BOOT2: &[u8] = include_bytes!("../../../../assets/boot_logo2.bmp");

pub(crate) struct Ssd1306Driver {
    count: u32,
    display: Ssd1306Async<I2CInterface<I2cDevice<'static, CriticalSectionRawMutex, I2c<'static, Async>>>, DisplaySize128x64, BufferedGraphicsModeAsync<DisplaySize128x64>>,
}

// impl Driver for Ssd1306Driver {}

impl  Ssd1306Driver {
    pub(crate) fn init(i2c_1: I2cDevice<'static, CriticalSectionRawMutex, I2c<'static, Async>>) -> Self {


        let i2c_disp = I2CDisplayInterface::new(i2c_1);


        let display = Ssd1306Async::new(
            i2c_disp,
            DisplaySize128x64,
            DisplayRotation::Rotate180,
        ).into_buffered_graphics_mode();


        Self {
            display,
            count: 0
        }

        // display.init().await.unwrap();

    }
}






#[async_trait(?Send)]
impl AsyncDisplay for Ssd1306Driver {
    async fn init(&mut self) {
        self.display.init().await.unwrap();
        let bmp = Bmp::from_slice(BOOT1).unwrap();
        let image = Image::with_center(&bmp, Point::new(64, 32));
        image.draw(&mut self.display).unwrap();
        self.display.flush().await.unwrap();
    }

    async fn draw(&mut self, angle: u8) {
        let bmp =
        if (self.count % 2) == 0 {
            Bmp::from_slice(BOOT1).unwrap()
        } else {
            Bmp::from_slice(BOOT2).unwrap()
        };

        let image = Image::with_center(&bmp, Point::new(64, 32));
        image.draw(&mut self.display).unwrap();
        self.display.flush().await.unwrap();
        self.count += 1;
    }
}
