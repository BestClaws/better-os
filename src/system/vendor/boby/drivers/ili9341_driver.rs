use alloc::boxed::Box;
use display_interface_spi::SPIInterface;
use crate::system::hal::display::AsyncDisplay;
use embassy_embedded_hal::shared_bus::blocking::spi::SpiDevice;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embedded_graphics::pixelcolor::Rgb565;
use embedded_graphics::prelude::{DrawTarget, RgbColor};
use embedded_hal::digital::OutputPin;
use esp_hal::{Async, Blocking};
use esp_hal::spi::master::Spi;
use ili9341::{Ili9341, Orientation};



pub(crate) struct Ili9341Driver {
}

impl Ili9341Driver {
    pub(crate) fn init(spi_1: SpiDevice<'static, CriticalSectionRawMutex, Spi<'static, Blocking>, impl OutputPin>, dc: impl OutputPin, r: impl OutputPin)-> Self {

        let iface = SPIInterface::new(spi_1, dc);
        let mut delay = embassy_time::Delay;


        let mut display = Ili9341::new(
            iface,
            r,
            &mut delay,
            Orientation::Landscape,
            ili9341::DisplaySize240x320,
        )
            .unwrap();

        let mut y: u64 = 0;

        loop {
            y+= 1;
            let x = (y % 256) as u8;
            display.clear(Rgb565::new(x, 255-x, (x as f32 / 2.0) as u8)).unwrap();

        }


        todo!()
    }

}

#[async_trait::async_trait(?Send)]
impl AsyncDisplay for Ili9341Driver {
    async fn init(&mut self) {
        todo!()
    }

    async fn draw(&mut self, buffer: &[u8]) {
        todo!()
    }

}