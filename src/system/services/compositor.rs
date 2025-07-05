use crate::system::hal::encoder::EncoderStateHandler;
use alloc::boxed::Box;
use defmt::info;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::mutex::Mutex;
use embassy_sync::signal::Signal;
use embassy_time::Duration;

pub static SIG_A: Signal<CriticalSectionRawMutex, Duration> = Signal::new();
pub static SIG_B: Signal<CriticalSectionRawMutex, Duration> = Signal::new();


// TODO: why is there a reference to driver in the task? get rid of this.
#[embassy_executor::task]
pub async fn compositor_service(x: &'static Mutex<CriticalSectionRawMutex, Box<dyn EncoderStateHandler>>) {
    
    loop {
        info!("Compositor service started");
        x.lock().await.get_state().unwrap();
        
    }

}



