use alloc::boxed::Box;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::mutex::Mutex;
use crate::system::hal::display::Display;
use crate::system::hal::encoder::{ EncoderStateHandler};

pub(crate) struct PlatformDevice<'s> {
    pub(crate) encoder: Option<&'s mut Mutex<CriticalSectionRawMutex,Box<dyn EncoderStateHandler>>>
    
}