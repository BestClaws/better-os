use alloc::boxed::Box;
use async_trait::async_trait;
use bt_hci::controller::ExternalController;
use esp_wifi::ble::controller::BleConnector;
use esp_wifi::EspWifiController;
use trouble_host::prelude::Peripheral;
use crate::system::hal::radio::AsyncRadio;
use esp_hal::peripherals::BT;
// TODO: hard assuming we are using ADC1, bad. even for a driver.


pub struct RadioDriver<'a> {
    hci: ExternalController<BleConnector<'a>, 20>,
}

impl<'a> RadioDriver<'a> {
    pub fn new(radio_init: EspWifiController<'a>, bt: BT<'a>) -> Self {

        let init  = Box::new(radio_init);
        let init = Box::leak(init);


        let connector = BleConnector::new(init, bt);
        let controller: ExternalController<_, 20> = ExternalController::new(connector);



        Self { hci: controller }
    }
}


#[async_trait(?Send)]
impl<'a> AsyncRadio<'a> for RadioDriver<'a> {

    async fn get_controller(&'a mut self) -> &mut ExternalController<BleConnector<'a>, 20> {
        &mut self.hci
    }

}
