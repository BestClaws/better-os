use crate::system::hal::rtc::{AsyncRtc, RtcDateTime};
use alloc::boxed::Box;
use defmt::warn;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::mutex::Mutex;
use embassy_time::{Duration, Timer};

pub use crate::system::hal::rtc::RtcError;

static RTC_STATE: Mutex<CriticalSectionRawMutex, Option<RtcDateTime>> = Mutex::new(None);

/// Returns the most recently sampled RTC value, if available.
pub async fn current_datetime() -> Option<RtcDateTime> {
    let state = RTC_STATE.lock().await;
    *state
}

#[embassy_executor::task]
pub(crate) async fn rtc_service(
    rtc: &'static Mutex<CriticalSectionRawMutex, Box<dyn AsyncRtc>>,
) {
    {
        let mut guard = rtc.lock().await;
        if let Err(err) = guard.init().await {
            warn!("RTC init failed: {:?}", err);
        }
    }

    loop {
        let sample = {
            let mut guard = rtc.lock().await;
            match guard.now().await {
                Ok(value) => Some(value),
                Err(err) => {
                    warn!("RTC read failed: {:?}", err);
                    None
                }
            }
        };

        if let Some(value) = sample {
            let mut state = RTC_STATE.lock().await;
            *state = Some(value);
        }

        Timer::after(Duration::from_secs(1)).await;
    }
}
