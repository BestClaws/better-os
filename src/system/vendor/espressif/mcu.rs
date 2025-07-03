use defmt::info;
use esp_hal::clock::CpuClock;
use esp_hal::peripherals::Peripherals;
use esp_hal::timer::systimer::SystemTimer;
use esp_hal::timer::timg::TimerGroup;

pub(crate) struct McuDriver {
    
    // include all peripherals. like radio ledc, spi i2c.
}

pub fn init() -> Peripherals {
    let config = esp_hal::Config::default().with_cpu_clock(CpuClock::max());
    esp_hal::init(config)

}
