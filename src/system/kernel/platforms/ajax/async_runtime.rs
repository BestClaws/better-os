
use esp_hal_embassy::TimeBase;

const LGC: &str = module_path!();

pub(crate) fn  init(time_base: impl TimeBase) {
    
    esp_hal_embassy::init(time_base);
    defmt::info!("{} Embassy initialized", LGC);
}