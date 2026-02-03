use crate::system::hal::display::AsyncDisplay;

use crate::system::kernel::platform::PlatformDevice;

use crate::system::vendor::chipone::co5300::Co5300;
use crate::system::vendor::espressif::mcu;
use alloc::boxed::Box;
use defmt::Format;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::mutex::Mutex;
use embedded_hal::digital::OutputPin;
use esp_hal::dma::{DmaRxBuf, DmaTxBuf};
use esp_hal::gpio::{Level, Output, OutputConfig};
use esp_hal::interrupt::software::SoftwareInterruptControl;
use esp_hal::spi::master::{Config, Spi};
use esp_hal::spi::Mode;
use esp_hal::time::Rate;
use esp_hal::timer::systimer::SystemTimer;
use esp_hal::{dma_buffers, Async};
use static_cell::StaticCell;

static SPI_BUS: StaticCell<Mutex<CriticalSectionRawMutex, Spi<Async>>> = StaticCell::new();

pub(crate) static DISPLAY: StaticCell<Mutex<CriticalSectionRawMutex, Box<dyn AsyncDisplay>>> =
    StaticCell::new();

pub(crate) fn init_device() -> PlatformDevice<'static> {
    let peripherals = mcu::init();
    let system_timer = SystemTimer::new(peripherals.SYSTIMER);
    let sw_interrupts = SoftwareInterruptControl::new(peripherals.SW_INTERRUPT);
    let st_alarm = system_timer.alarm0;
    let SoftwareInterruptControl {
        software_interrupt0: sw_int0,
        ..
    } = sw_interrupts;
    crate::system::kernel::platforms::ajax::async_runtime::init(st_alarm, sw_int0);

    let (rx_buffer, rx_descriptors, tx_buffer, tx_descriptors) = dma_buffers!(16384);
    let dma_rx_buf = DmaRxBuf::new(rx_descriptors, rx_buffer).unwrap();
    let dma_tx_buf = DmaTxBuf::new(tx_descriptors, tx_buffer).unwrap();

    let lcd_spi = Spi::new(
        peripherals.SPI2,
        Config::default()
            .with_frequency(Rate::from_mhz(70_u32))
            .with_mode(Mode::_0),
    )
    .unwrap()
    .with_sio0(peripherals.GPIO1)
    .with_sio1(peripherals.GPIO2)
    .with_sio2(peripherals.GPIO3)
    .with_sio3(peripherals.GPIO4)
    .with_cs(peripherals.GPIO5)
    .with_sck(peripherals.GPIO0)
    .with_dma(peripherals.DMA_CH0)
    .with_buffers(dma_rx_buf, dma_tx_buf)
    .into_async();

    // Initialize Co5300 driver
    let reset_pin = Output::new(peripherals.GPIO11, Level::High, OutputConfig::default());
    let mut display = Co5300::new(lcd_spi, reset_pin);

    PlatformDevice {
        display: Some(DISPLAY.init(Mutex::new(Box::new(display)))),
    }
}

#[derive(Debug, Format)]
struct NothingPin;

impl embedded_hal::digital::ErrorType for NothingPin {
    type Error = core::convert::Infallible;
}

impl OutputPin for NothingPin {
    #[inline]
    fn set_low(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }

    #[inline]
    fn set_high(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }
}
