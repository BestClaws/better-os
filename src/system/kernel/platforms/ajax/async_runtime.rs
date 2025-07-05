use defmt::info;
use embassy_time::Instant;
use esp_hal_embassy::TimeBase;

// const LGC: &str = module_path!();

pub(crate) fn  init(time_base: impl TimeBase) {
    
    esp_hal_embassy::init(time_base);

    info!("[{}s] async runtime initialized", Instant::now().as_millis() as f32 / 1000f32);
    // defmt::info!("{} Embassy initialized", LGC);
}