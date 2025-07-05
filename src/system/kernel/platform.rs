use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::mutex::Mutex;
use crate::system::hal::display::Display;
use crate::system::hal::encoder::Encoder;

pub(crate) struct PlatformDevice<'s, ENCODER: Encoder, DISPLAY: Display> {
    pub(crate) encoder: Option<&'s mut Mutex<CriticalSectionRawMutex,ENCODER>>,
    pub(crate) display: Option<&'s mut Mutex<CriticalSectionRawMutex, DISPLAY>>
    
}