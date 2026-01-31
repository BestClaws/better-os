use crate::libs::http;
use crate::system::services::hps_service::wait_for_hps_ready;
use crate::system::services::tasks::rtc_srv::{set_datetime_from_worldtime, RtcServiceError};
use defmt::{info, warn};
use embassy_time::{Duration, Timer};

const WORLD_TIME_URL: &str = "https://worldtimeapi.org/api/timezone/Asia/Kolkata";
const RETRY_DELAY_SECS: u64 = 30;
const QUICK_RETRY_DELAYS_MS: [u64; 5] = [500, 1000, 2000, 3000, 4000];

#[embassy_executor::task]
pub(crate) async fn rtc_sync_service() {
    info!("RTC sync service starting");

    wait_for_hps_ready().await;
    info!("RTC sync: HPS ready, attempting time fetch");

    let client = http::Client::new();

    loop {
        let mut quick_retry_index: usize = 0;

        loop {
            if try_sync_once(&client).await {
                return;
            }

            if let Some(delay_ms) = QUICK_RETRY_DELAYS_MS.get(quick_retry_index) {
                info!("RTC sync: quick retry in {} ms", delay_ms);
                Timer::after(Duration::from_millis(*delay_ms)).await;
                quick_retry_index += 1;
                continue;
            }

            break;
        }

        info!("RTC sync: retrying in {} seconds", RETRY_DELAY_SECS);
        Timer::after(Duration::from_secs(RETRY_DELAY_SECS)).await;
    }
}

async fn try_sync_once(client: &http::Client) -> bool {
    match client.get_secure(WORLD_TIME_URL).send().await {
        Ok(response) => {
            if !response.is_success() {
                warn!("RTC sync: HTTP {} from worldtime API", response.status());
                return false;
            }

            match response.text() {
                Ok(body) => match set_datetime_from_worldtime(body.as_bytes()).await {
                    Ok(()) => {
                        info!("RTC sync: time synchronized successfully");
                        true
                    }
                    Err(RtcServiceError::Parse) => {
                        warn!("RTC sync: failed to parse worldtime payload");
                        false
                    }
                    Err(err) => {
                        warn!("RTC sync: failed to apply worldtime data: {:?}", err);
                        false
                    }
                },
                Err(_) => {
                    warn!("RTC sync: failed to read worldtime response body");
                    false
                }
            }
        }
        Err(err) => {
            warn!("RTC sync: request failed: {:?}", err);
            false
        }
    }
}
