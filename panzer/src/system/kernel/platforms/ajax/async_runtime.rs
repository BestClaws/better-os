use defmt::info;
use embassy_time::Instant;
use esp_hal::interrupt::software::SoftwareInterrupt;
use esp_rtos::TimerSource;

// const LGC: &str = module_path!();

pub(crate) fn init(timer: impl TimerSource, interrupt: SoftwareInterrupt<'static, 0>) {
    esp_rtos::start(timer, interrupt);

    info!(
        "[{}s] async runtime initialized",
        Instant::now().as_millis() as f32 / 1000f32
    );
    // defmt::info!("{} Embassy initialized", LGC);
}
