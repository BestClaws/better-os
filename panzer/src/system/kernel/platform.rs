use crate::system::hal::display::AsyncDisplay;
use alloc::boxed::Box;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::mutex::Mutex;

pub(crate) struct PlatformDevice<'s> {
    pub(crate) display: Option<&'s mut Mutex<CriticalSectionRawMutex, Box<dyn AsyncDisplay>>>,
    pub(crate) touch: Option<crate::system::vendor::chipone::cst816s::Cst816s<esp_hal::i2c::master::I2c<'static, esp_hal::Async>>>,
}
