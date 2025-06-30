use esp_hal::peripherals;
use esp_hal::peripherals::SYSTIMER;
use esp_hal::timer::systimer::SystemTimer;
use esp_hal::timer::timg::TimerGroup;
const LGC: &str = module_path!();

pub(crate) fn init(system_timer: SYSTIMER) {
    
    let timer0 = SystemTimer::new(system_timer);
    esp_hal_embassy::init(timer0.alarm0);
    defmt::info!("{} Embassy initialized", LGC);
}