
use esp_hal::peripherals::{RADIO_CLK, RNG, TIMG0};
use esp_hal::rng::Rng;
use esp_hal::timer::timg::TimerGroup;
use esp_wifi::EspWifiController;

pub struct RadioDriver<'d> {
    controller: EspWifiController<'d>
}


impl <'d> RadioDriver<'d> {
    pub fn new(rng: RNG, timer_group: TIMG0<'d>, radio_clk: RADIO_CLK<'d>) -> RadioDriver<'d> {
        let rng = Rng::new(rng);
        let timer1 = TimerGroup::new(timer_group);
        let esp_wifi_controller = esp_wifi::init(
            timer1.timer0,
            rng.clone(),
            radio_clk,
        ).unwrap();


        Self { controller: esp_wifi_controller }
    }


}

