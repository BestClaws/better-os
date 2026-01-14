use crate::libs::http;
use crate::system::services::hps_service::wait_for_hps_ready;
use crate::system::services::rtc_srv::{set_datetime_from_worldtime, RtcServiceError};
use defmt::{info, warn};
use embassy_time::{Duration, Timer};

const WORLD_TIME_URL: &str = "https://worldtimeapi.org/api/timezone/Asia/Kolkata";
const RETRY_DELAY_SECS: u64 = 30;

#[embassy_executor::task]
pub(crate) async fn rtc_sync_service() {
    info!("RTC sync service starting");

    wait_for_hps_ready().await;
    info!("RTC sync: HPS ready, attempting time fetch");

    let client = http::Client::new();

    loop {
        match client.get_secure(WORLD_TIME_URL).send().await {
            Ok(response) => {
                if !response.is_success() {
                    warn!("RTC sync: HTTP {} from worldtime API", response.status());
                } else if let Ok(body) = response.text() {
                    match set_datetime_from_worldtime(body.as_bytes()).await {
                        Ok(()) => {
                            info!("RTC sync: time synchronized successfully");
                            return;
                        }
                        Err(RtcServiceError::Parse) => {
                            warn!("RTC sync: failed to parse worldtime payload");
                        }
                    }
                } else {
                    warn!("RTC sync: failed to read worldtime response body");
                }
            }
            Err(err) => {
                warn!("RTC sync: request failed: {:?}", err);
            }
        }

        info!("RTC sync: retrying in {} seconds", RETRY_DELAY_SECS);
        Timer::after(Duration::from_secs(RETRY_DELAY_SECS)).await;
    }
}
