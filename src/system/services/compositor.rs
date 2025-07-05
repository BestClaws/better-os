use core::cell::Cell;
use defmt::{info, Debug2Format};
use embassy_futures::select::select;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::signal::Signal;
use embassy_time::{Duration, Timer};
use esp_hal::gpio::{Level, Output, OutputConfig};
use esp_hal::peripherals::GPIO7;
use crate::system::kernel::platforms::ajax::device::PLATFORM_DEVICE;

pub static SIG_A: Signal<CriticalSectionRawMutex, Duration> = Signal::new();
pub static SIG_B: Signal<CriticalSectionRawMutex, Duration> = Signal::new();


#[embassy_executor::task]
pub async fn compositor_service() {
    
    
    PLATFORM_DEVICE.
   
}



