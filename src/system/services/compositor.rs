use core::cell::Cell;
use defmt::{info, Debug2Format};
use embassy_futures::select::select;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::mutex::Mutex;
use embassy_sync::signal::Signal;
use embassy_time::{Duration, Timer};

use crate::system::kernel::platforms::ajax::device::{AjaxDev};


pub static SIG_A: Signal<CriticalSectionRawMutex, Duration> = Signal::new();
pub static SIG_B: Signal<CriticalSectionRawMutex, Duration> = Signal::new();


#[embassy_executor::task]
pub async fn compositor_service(x: &'static Mutex<CriticalSectionRawMutex, AjaxDev>) {
    
    
   
}



