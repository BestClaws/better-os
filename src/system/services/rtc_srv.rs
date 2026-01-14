use crate::system::hal::rtc::{AsyncRtc, RtcDateTime};
use alloc::boxed::Box;
use defmt::{info, warn};
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::mutex::Mutex;
use embassy_time::{Duration, Timer};

pub use crate::system::hal::rtc::RtcError;

static RTC_STATE: Mutex<CriticalSectionRawMutex, Option<RtcDateTime>> = Mutex::new(None);

const DEFAULT_BOOT_DATETIME: RtcDateTime = RtcDateTime::new(2026, 1, 1, 0, 0, 0, 0);

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

    let mut integrity_warned = false;
    let mut recovery_attempts: u8 = 0;

    loop {
        let sample = {
            let mut guard = rtc.lock().await;
            match guard.now().await {
                Ok(value) => Some(value),
                Err(err) => {
                    match err {
                        RtcError::InvalidData => {
                            if !integrity_warned {
                                warn!("RTC clock integrity lost; applying default timestamp");
                                integrity_warned = true;
                            }

                            if recovery_attempts < 5 {
                                recovery_attempts += 1;
                                if let Err(write_err) = guard.set(&DEFAULT_BOOT_DATETIME).await {
                                    warn!("RTC recovery write failed: {:?}", write_err);
                                } else {
                                    info!("RTC default time written" );
                                }
                            }
                        }
                        other => warn!("RTC read failed: {:?}", other),
                    }
                    None
                }
            }
        };

        if let Some(value) = sample {
            if integrity_warned {
                info!("RTC recovered; now {:?}-{:?}-{:?} {:?}:{:?}:{:?}", value.year, value.month, value.day, value.hour, value.minute, value.second);
            }
            integrity_warned = false;
            recovery_attempts = 0;
            let mut state = RTC_STATE.lock().await;
            *state = Some(value);
        }

        Timer::after(Duration::from_secs(1)).await;
    }
}
