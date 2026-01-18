use crate::system::hal::ambience::AsyncAmbientSensor;
use crate::system::hal::audio::AsyncAudioSink;
use crate::system::hal::battery::AsyncBattery;
use crate::system::hal::button::{AsyncButton, ButtonDriver};
use crate::system::hal::display::AsyncDisplay;
use crate::system::hal::display::PixelFormat;
use crate::system::hal::encoder::AsyncEncoder;
use crate::system::hal::imu::AsyncGyroAccelerometer;
use crate::system::hal::radio::AsyncRadio;
use crate::system::hal::rtc::AsyncRtc;
use crate::system::hal::touch::AsyncTouch;
use crate::system::hal::vibrator::AsyncVibrator;
use crate::system::kernel::platform::PlatformDevice;
use crate::system::ui::windowing::WindowHandle;
use crate::system::vendor::boby::drivers::ambient_sensor::AmbientSensorDriver;
use crate::system::vendor::boby::drivers::audio::I2sAudioDriver;
use crate::system::vendor::boby::drivers::es8311;
use crate::system::vendor::boby::drivers::battery::BatteryDriver;
use crate::system::vendor::boby::drivers::encoder::EncoderDriver;
use crate::system::vendor::boby::drivers::ft5336::FT5336;
use crate::system::vendor::boby::drivers::pcf85063::Pcf85063;
use crate::system::vendor::boby::drivers::qmi8658c::Qmi8658C;
use crate::system::vendor::boby::drivers::vibrator::VibratorDriver;
use crate::system::vendor::boby::drivers::xpt2046::XPT2046;
use crate::system::vendor::chipone::co5300::Co5300;
use crate::system::vendor::espressif::drivers::radio_driver::RadioDriver;
use crate::system::vendor::espressif::mcu;
use alloc::boxed::Box;
use core::cell::RefCell;
use defmt::{info, warn, Format};
use embassy_embedded_hal::shared_bus::asynch::i2c::I2cDevice;
use embassy_embedded_hal::shared_bus::asynch::spi::SpiDeviceWithConfig;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::mutex::Mutex;
use embassy_time::Timer;
use embedded_hal::digital::OutputPin;
use esp_hal::analog::adc::{Adc, AdcConfig, Attenuation};
use esp_hal::delay::Delay;
use esp_hal::dma::{DmaRxBuf, DmaTxBuf};
use esp_hal::dma_circular_descriptors;
use esp_hal::i2s::master::{Channels, Config as I2sConfig, DataFormat as I2sDataFormat, I2s};
use esp_hal::gpio::{Level, Output, OutputConfig};
use esp_hal::interrupt::software::SoftwareInterruptControl;
use esp_hal::peripherals::ADC1;
use esp_hal::spi::master::{Config, Spi};
use esp_hal::spi::Mode;
use esp_hal::time::Rate;
use esp_hal::timer::systimer::SystemTimer;
use esp_hal::{dma_buffers, Async, Blocking};
use esp_hal::{
    gpio::{Input, InputConfig, Pull},
    i2c::master::I2c,
};
use static_cell::StaticCell;

static SPI_BUS: StaticCell<Mutex<CriticalSectionRawMutex, Spi<Async>>> = StaticCell::new();
static I2C_BUS: StaticCell<Mutex<CriticalSectionRawMutex, I2c<Async>>> = StaticCell::new();

pub(crate) static ENCODER: StaticCell<Mutex<CriticalSectionRawMutex, Box<dyn AsyncEncoder>>> =
    StaticCell::new();
pub(crate) static VIBRATOR: StaticCell<Mutex<CriticalSectionRawMutex, Box<dyn AsyncVibrator>>> =
    StaticCell::new();
pub(crate) static APP_SWITCH_BUTTON: StaticCell<
    Mutex<CriticalSectionRawMutex, Box<dyn AsyncButton>>,
> = StaticCell::new();
pub(crate) static DISPLAY: StaticCell<Mutex<CriticalSectionRawMutex, Box<dyn AsyncDisplay>>> =
    StaticCell::new();
pub(crate) static TOUCH: StaticCell<Mutex<CriticalSectionRawMutex, Box<dyn AsyncTouch>>> =
    StaticCell::new();
pub(crate) static BATTERY: StaticCell<Mutex<CriticalSectionRawMutex, Box<dyn AsyncBattery>>> =
    StaticCell::new();
pub(crate) static GYRO_ACCELEROMETER: StaticCell<
    Mutex<CriticalSectionRawMutex, Box<dyn AsyncGyroAccelerometer>>,
> = StaticCell::new();
pub(crate) static ADC_SHARED: StaticCell<Mutex<CriticalSectionRawMutex, Adc<ADC1, Async>>> =
    StaticCell::new();
pub(crate) static RADIO: StaticCell<Mutex<CriticalSectionRawMutex, Box<dyn AsyncRadio>>> =
    StaticCell::new();
pub(crate) static RTC: StaticCell<Mutex<CriticalSectionRawMutex, Box<dyn AsyncRtc>>> =
    StaticCell::new();
pub(crate) static AUDIO: StaticCell<Mutex<CriticalSectionRawMutex, Box<dyn AsyncAudioSink>>> =
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

    let mut delay = Delay::new();

    let mut pa_enable = Output::new(peripherals.GPIO6, Level::Low, OutputConfig::default());

    let mut i2c = I2c::new(
        peripherals.I2C0,
        esp_hal::i2c::master::Config::default().with_frequency(Rate::from_khz(400)),
    )
    .unwrap()
    .with_sda(peripherals.GPIO8)
    .with_scl(peripherals.GPIO7);

    if let Err(_err) = es8311::init_codec(&mut i2c, &mut pa_enable, &mut delay, 48_000, false) {
        warn!("ES8311 initialization failed; audio output unavailable");
    }

    let i2c = i2c.into_async();

    let i2c = Mutex::new(i2c);
    let i2c = I2C_BUS.init(i2c);
    let i2c_1: I2cDevice<'static, CriticalSectionRawMutex, I2c<'static, Async>> =
        I2cDevice::new(i2c);
    let i2c_2: I2cDevice<'static, CriticalSectionRawMutex, I2c<'static, Async>> =
        I2cDevice::new(i2c);
    let i2c_3: I2cDevice<'static, CriticalSectionRawMutex, I2c<'static, Async>> =
        I2cDevice::new(i2c);

    // Touch reset pin
    let touch_reset_pin = Output::new(peripherals.GPIO10, Level::High, OutputConfig::default());

    let mut touch = FT5336::new(i2c_1, Some(touch_reset_pin));
    let accel = Qmi8658C::new(i2c_2);
    let rtc = Pcf85063::new(i2c_3);

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

    // Reset pin
    let reset_pin = Output::new(peripherals.GPIO11, Level::High, OutputConfig::default());

    // Initialize Co5300 driver
    let mut display = Co5300::new(lcd_spi, reset_pin);

    let app_switch_pin = Input::new(
        peripherals.GPIO9,
        InputConfig::default().with_pull(Pull::Up),
    );

    let app_switch = ButtonDriver::new(app_switch_pin);

    let radio_driver = RadioDriver::new(peripherals.BT);

    const AUDIO_BUFFER_BYTES: usize = 2048;
    let (_, audio_tx_descriptors) = dma_circular_descriptors!(AUDIO_BUFFER_BYTES);
    let I2s { i2s_rx, i2s_tx, .. } = I2s::new(
        peripherals.I2S0,
        peripherals.DMA_CH1,
        I2sConfig::new_tdm_philips()
            .with_sample_rate(Rate::from_hz(48_000))
            .with_channels(Channels::STEREO)
            .with_data_format(I2sDataFormat::Data16Channel16),
    )
    .unwrap();
    drop(i2s_rx);

    let audio_tx = i2s_tx
        .with_bclk(peripherals.GPIO20)
        .with_ws(peripherals.GPIO22)
        .with_dout(peripherals.GPIO23)
        .build(audio_tx_descriptors);

    let audio_driver = I2sAudioDriver::new(audio_tx);

    PlatformDevice {
        touch: Some(TOUCH.init(Mutex::new(Box::new(touch)))),
        display: Some(DISPLAY.init(Mutex::new(Box::new(display)))),
        radio: Some(RADIO.init(Mutex::new(Box::new(radio_driver)))),
        gyro_accelerometer: Some(GYRO_ACCELEROMETER.init(Mutex::new(Box::new(accel)))),
        button: None,
        app_switch_button: Some(APP_SWITCH_BUTTON.init(Mutex::new(Box::new(app_switch)))),
        rtc: Some(RTC.init(Mutex::new(Box::new(rtc)))),
        audio: Some(AUDIO.init(Mutex::new(Box::new(audio_driver)))),
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
