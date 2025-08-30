use crate::system::kernel::platform::PlatformDevice;
use crate::system::vendor::espressif::mcu;
use alloc::boxed::Box;
use core::cell::RefCell;
use defmt::{info, Format};
use crate::system::hal::ambience::AsyncAmbientSensor;
use crate::system::hal::battery::AsyncBattery;
use crate::system::hal::button::{AsyncButton, ButtonDriver};
use crate::system::hal::display::AsyncDisplay;
use crate::system::hal::encoder::AsyncEncoder;
use crate::system::hal::imu::AsyncGyroAccelerometer;
use crate::system::hal::radio::AsyncRadio;
use crate::system::vendor::boby::drivers::ambient_sensor::AmbientSensorDriver;
use crate::system::vendor::boby::drivers::battery::BatteryDriver;
use crate::system::vendor::boby::drivers::encoder::EncoderDriver;
use crate::system::vendor::boby::drivers::ssd1306::Ssd1306Driver;
use crate::system::vendor::espressif::drivers::radio_driver::{RadioDriver};
use crate::system::vendor::invensense::drivers::mpu6050::sensor::MPU6050;
use embassy_embedded_hal::shared_bus::asynch::i2c::I2cDevice;
use embassy_embedded_hal::shared_bus::asynch::spi::{SpiDevice, SpiDeviceWithConfig};
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::mutex::Mutex;
use embassy_time::Timer;
use embedded_graphics::mono_font::ascii::FONT_10X20;
use embedded_graphics::mono_font::MonoTextStyle;
use embedded_graphics::text::{Alignment, LineHeight, TextStyleBuilder};
use embedded_graphics_core::pixelcolor::Rgb888;
use embedded_graphics_core::prelude::{DrawTarget, RgbColor};
use embedded_hal::delay::DelayNs;
use embedded_hal::digital::OutputPin;
use esp_hal::analog::adc::{Adc, AdcConfig, Attenuation};
// use esp_hal::peripherals::ADC1;
use esp_hal::time::Rate;
use esp_hal::timer::systimer::SystemTimer;
use esp_hal::timer::timg::TimerGroup;
use esp_hal::{dma_buffers, Async, Blocking};
use esp_hal::{
    gpio::{Input, InputConfig, Pull},
    i2c::master::I2c,
};
use esp_hal::delay::Delay;
use esp_hal::dma::{DmaRxBuf, DmaTxBuf};
use esp_hal::gpio::{AnyPin, Level, Output, OutputConfig};
use esp_hal::spi::master::{Config, Spi};
use esp_hal::spi::Mode;
use static_cell::StaticCell;
use crate::system::hal::touch::AsyncTouch;
use crate::system::hal::vibrator::AsyncVibrator;
use crate::system::ui::window::WindowHandle;
use crate::system::vendor::boby::drivers::ili9341::driver::Ili9341Driver;
use crate::system::vendor::boby::drivers::vibrator::VibratorDriver;
use crate::system::vendor::boby::drivers::xpt2046::XPT2046;
use esp_hal::peripherals::ADC1;
use crate::system::kernel::platforms::ajax::display_driver::{ResetDriver, Ws43AmoledDriver};
use crate::system::kernel::platforms::ajax::driver_lib::{framebuffer_size, ColorMode, DisplaySize, Sh8601Driver};
use crate::system::vendor::boby::drivers::ft5336::FT5336;

static SPI_BUS: StaticCell<Mutex<CriticalSectionRawMutex, Spi<Async>>> = StaticCell::new();

static I2C_BUS: StaticCell<Mutex<CriticalSectionRawMutex, I2c<Async>>> = StaticCell::new();

pub(crate) static ENCODER: StaticCell<Mutex<CriticalSectionRawMutex, Box<dyn AsyncEncoder>>> =
    StaticCell::new();
pub(crate) static VIBRATOR: StaticCell<Mutex<CriticalSectionRawMutex, Box<dyn AsyncVibrator>>> =
    StaticCell::new();

pub(crate) static BUTTON: StaticCell<Mutex<CriticalSectionRawMutex, Box<dyn AsyncButton>>> =
    StaticCell::new();
pub(crate) static DISPLAY: StaticCell<Mutex<CriticalSectionRawMutex, Box<dyn AsyncDisplay>>> =
    StaticCell::new();

pub(crate) static TOUCH: StaticCell<Mutex<CriticalSectionRawMutex, Box<dyn AsyncTouch>>> =
    StaticCell::new();
pub(crate) static BATTERY: StaticCell<Mutex<CriticalSectionRawMutex, Box<dyn AsyncBattery>>> =
    StaticCell::new();
pub(crate) static AMBIENT_SENSOR: StaticCell<
    Mutex<CriticalSectionRawMutex, Box<dyn AsyncAmbientSensor>>,
> = StaticCell::new();
pub(crate) static GYRO_ACCELEROMETER: StaticCell<
    Mutex<CriticalSectionRawMutex, Box<dyn AsyncGyroAccelerometer>>,
> = StaticCell::new();
pub(crate) static ADC_SHARED: StaticCell<Mutex<CriticalSectionRawMutex, Adc<ADC1, Async>>> =
    StaticCell::new();

pub(crate) static RADIO: StaticCell<Mutex<CriticalSectionRawMutex, Box<dyn AsyncRadio>>> =
    StaticCell::new();

pub(crate) fn init_device() -> PlatformDevice<'static> {
    // initialize mcu device hal
    let peripherals = mcu::init();

    // TODO: this should be something that should be present in the kernel.
    // initialize async runtime
    // the core model of multitasking.
    let system_timer = SystemTimer::new(peripherals.SYSTIMER);
    let st_alarm = system_timer.alarm0;
    crate::system::kernel::platforms::ajax::async_runtime::init(st_alarm);



    // INIT I2C BUS
    let i2c = I2c::new(
        peripherals.I2C0,
        esp_hal::i2c::master::Config::default().with_frequency(Rate::from_khz(400)),
    )
    .unwrap()
    .with_sda(peripherals.GPIO18)
    .with_scl(peripherals.GPIO8)
    .into_async();


    let i2c = Mutex::new(i2c);
    let i2c = I2C_BUS.init(i2c);
    let i2c_1: I2cDevice<'static, CriticalSectionRawMutex, I2c<'static, Async>> =
        I2cDevice::new(i2c);



    // INIT TOUCH
    let touch = FT5336::new(i2c_1);

    // --- DMA Buffers for SPI ---
    let (rx_buffer, rx_descriptors, tx_buffer, tx_descriptors) = dma_buffers!(16384);
    let dma_rx_buf = DmaRxBuf::new(rx_descriptors, rx_buffer).unwrap();
    let dma_tx_buf = DmaTxBuf::new(tx_descriptors, tx_buffer).unwrap();


    let lcd_spi = Spi::new(
        peripherals.SPI2,
        Config::default()
            .with_frequency(Rate::from_mhz(40_u32))
            .with_mode(Mode::_0),
    )
        .unwrap()
        .with_sio0(peripherals.GPIO4)
        .with_sio1(peripherals.GPIO5)
        .with_sio2(peripherals.GPIO6)
        .with_sio3(peripherals.GPIO7)
        .with_cs(peripherals.GPIO10)
        .with_sck(peripherals.GPIO11)
        .with_dma(peripherals.DMA_CH0)
        .with_buffers(dma_rx_buf, dma_tx_buf);

    let delay = &mut embassy_time::Delay;


    let reset_pin    = Output::new(peripherals.GPIO3, Level::High, OutputConfig::default());
    let reset = ResetDriver::new(reset_pin, delay);

    // Initialize display driver for the Waveshare 1.8" AMOLED display
    let ws_driver = Ws43AmoledDriver::new(lcd_spi);

    // Set up the display size
    const DISPLAY_SIZE: DisplaySize = DisplaySize::new(466, 100);

    // Calculate framebuffer size based on the display size and color mode
    const FB_SIZE: usize = framebuffer_size(DISPLAY_SIZE, ColorMode::Rgb888);

    let delay = &mut embassy_time::Delay;

    let display_res = Sh8601Driver::new_heap::<_, FB_SIZE>(
        ws_driver,
        reset,
        ColorMode::Rgb888,
        DISPLAY_SIZE,
        delay,
    );
    let mut display = match display_res {
        Ok(d) => {
            info!("display read");

            d
        }
        Err(e) => {
            loop {}
        }
    };


    let delay = &mut embassy_time::Delay;

    info!("near clear");
    display.clear(Rgb888::WHITE).unwrap();

    info!("after clear");

    loop {
        delay.delay_ms(1000_u32);
    }


    // let sclk = peripherals.GPIO6; // SCLK
    // let miso = peripherals.GPIO2; // MISO
    // let mosi = peripherals.GPIO7; // MOSI
    //
    // let mut spi = Spi::new(
    //     peripherals.SPI2,
    //     Config::default()
    //         .with_frequency(Rate::from_mhz(40))
    //         .with_mode(Mode::_0),
    // )
    //     .unwrap()
    //     .with_sck(sclk)
    //     .with_mosi(mosi)
    //     .with_miso(miso)
    //     .into_async();
    //
    // let spi = Mutex::new(spi);
    // let spi = SPI_BUS.init(spi);
    //
    // // Display control pins from DTS
    // let dc    = Output::new(peripherals.GPIO4, Level::Low, OutputConfig::default());
    // let reset = Output::new(peripherals.GPIO5, Level::Low, OutputConfig::default());
    // let cs_display = Output::new(peripherals.GPIO3, Level::High, OutputConfig::default());
    //
    // // Touch control pins from DTS
    // let touch_irq = Input::new(peripherals.GPIO9, InputConfig::default().with_pull(Pull::Up));
    // let cs_touch  = Output::new(peripherals.GPIO1, Level::High, OutputConfig::default());
    //
    // // SPI devices
    // let spi_display = SpiDeviceWithConfig::new(
    //     spi,
    //     cs_display,
    //     Config::default().with_frequency(Rate::from_mhz(60)),
    // );
    // let display = Ili9341Driver::new(spi_display, dc, reset);
    //
    // let spi_touch = SpiDeviceWithConfig::new(
    //     spi,
    //     cs_touch,
    //     Config::default().with_frequency(Rate::from_mhz(2)),
    // );
    // let touch = XPT2046::new(spi_touch, touch_irq);
    //
    //





    PlatformDevice {
        // encoder: Some(ENCODER.init(Mutex::new(Box::new(encoder)))),
        // vibrator: Some(VIBRATOR.init(Mutex::new(Box::new(vibrator)))),
        // display: Some(DISPLAY.init(Mutex::new(Box::new(display)))),
        touch: Some(TOUCH.init(Mutex::new(Box::new(touch)))),
        // button: Some(BUTTON.init(Mutex::new(Box::new(button)))),
        // battery: Some(BATTERY.init(Mutex::new(Box::new(battery)))),
        // ambient_sensor: Some(AMBIENT_SENSOR.init(Mutex::new(Box::new(ambient_sensor)))),
        // gyro_accelerometer: Some(GYRO_ACCELEROMETER.init(Mutex::new(Box::new(gyro_accelerometer)))),
        // radio: Some(RADIO.init(Mutex::new(Box::new(radio_driver)))),

    }
}


#[derive(Debug, Format)]
struct NothingPin;


impl embedded_hal::digital::ErrorType for NothingPin { type Error = core::convert::Infallible; }

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


