use defmt::info;
use embassy_time::Timer;

#[embassy_executor::task]
pub async fn ticker() {
    loop {
        info!("Tick {:?}s", embassy_time::Instant::now().as_secs());
        Timer::after_secs(1).await;
    }
}