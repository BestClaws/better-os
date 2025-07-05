use alloc::boxed::Box;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::mutex::Mutex;
use crate::system::hal::display::AsyncDisplay;
use crate::system::hal::encoder::{ AsyncEncoder};

pub(crate) struct PlatformDevice<'s> {
    pub(crate) encoder: Option<&'s mut Mutex<CriticalSectionRawMutex,Box<dyn AsyncEncoder>>>,
    pub(crate) display: Option<&'s mut Mutex<CriticalSectionRawMutex,Box<dyn AsyncDisplay>>>
    
}