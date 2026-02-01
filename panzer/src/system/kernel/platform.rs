use crate::system::hal::display::AsyncDisplay;
use alloc::boxed::Box;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::mutex::Mutex;

pub(crate) struct PlatformDevice<'s> {
    pub(crate) display: Option<&'s mut Mutex<CriticalSectionRawMutex, Box<dyn AsyncDisplay>>>,
}
