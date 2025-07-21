use alloc::boxed::Box;
use display_interface_spi::SPIInterface;
use crate::system::hal::display::AsyncDisplay;
use embassy_embedded_hal::shared_bus::asynch::spi::SpiDevice;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embedded_graphics::pixelcolor::Rgb565;
use embedded_graphics::prelude::{DrawTarget, RgbColor};
use embedded_hal::digital::OutputPin;
use esp_hal::{Async};
use esp_hal::gpio::AnyPin;
use esp_hal::spi::master::Spi;
use crate::system::vendor::boby::drivers::ili9341::driver::{DisplaySize240x320, Ili9341Async, Orientation};

pub(crate) struct Ili9341Driver<CS: OutputPin, DC: OutputPin, RESET: OutputPin> {
    display:  Ili9341Async<SPIInterface<SpiDevice<'static, CriticalSectionRawMutex, Spi<'static, Async>, CS>, DC>, RESET>
}



impl<CS: OutputPin, DC: OutputPin, RESET: OutputPin> Ili9341Driver<CS, DC, RESET>{

    pub(crate) fn init(spi: SpiDevice<'static, CriticalSectionRawMutex, Spi<'static, Async>, CS>, dc: DC, r: RESET)

        -> Self {

        let interface = SPIInterface::new(spi, dc);
        let mut delay = embassy_time::Delay;


        let display = Ili9341Async::new_instance(
            interface,
            r,
            &mut delay,
            Orientation::Landscape,
            DisplaySize240x320,
        );

        Self {
            display
        }


    }

}

#[async_trait::async_trait(?Send)]
impl<CS: OutputPin, DC: OutputPin, RESET: OutputPin>  AsyncDisplay for Ili9341Driver<CS, DC, RESET> {
    async fn init(&mut self) {
        todo!()
    }

    async fn draw(&mut self, buffer: &[u8]) {
        self.display.clear_screen(2016).await.unwrap();
    }


    async fn clear(&mut self, color: u16) {
        self.display.clear_screen(color).await.unwrap();
    }

}