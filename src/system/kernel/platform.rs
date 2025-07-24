use alloc::boxed::Box;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::mutex::Mutex;
use crate::system::hal::ambience::AsyncAmbientSensor;
use crate::system::hal::battery::AsyncBattery;
use crate::system::hal::button::AsyncButton;
use crate::system::hal::display::AsyncDisplay;
use crate::system::hal::encoder::{ AsyncEncoder};
use crate::system::hal::imu::AsyncGyroAccelerometer;
use crate::system::hal::radio::AsyncRadio;
use crate::system::hal::touch::AsyncTouch;
use crate::system::hal::vibrator::AsyncVibrator;

pub(crate) struct PlatformDevice<'s> {
    // pub(crate) encoder: Option<&'s mut Mutex<CriticalSectionRawMutex,Box<dyn AsyncEncoder>>>,
    // pub(crate) vibrator: Option<&'s mut Mutex<CriticalSectionRawMutex,Box<dyn AsyncVibrator>>>,
    pub(crate) display: Option<&'s mut Mutex<CriticalSectionRawMutex,Box<dyn AsyncDisplay>>>,
    // pub(crate) touch: Option<&'s mut Mutex<CriticalSectionRawMutex,Box<dyn AsyncTouch>>>,
    // pub(crate) button: Option<&'s mut Mutex<CriticalSectionRawMutex,Box<dyn AsyncButton>>>,
    // pub(crate) battery: Option<&'s mut Mutex<CriticalSectionRawMutex,Box<dyn AsyncBattery>>>,
    // pub(crate) ambient_sensor: Option<&'s mut Mutex<CriticalSectionRawMutex,Box<dyn AsyncAmbientSensor>>>,
    // pub(crate) gyro_accelerometer: Option<&'s mut Mutex<CriticalSectionRawMutex,Box<dyn AsyncGyroAccelerometer>>>,
    pub(crate) radio: Option<&'s mut Mutex<CriticalSectionRawMutex,Box<dyn AsyncRadio>>>,
    
}