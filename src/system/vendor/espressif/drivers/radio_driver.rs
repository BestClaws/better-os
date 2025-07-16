use alloc::boxed::Box;
use async_trait::async_trait;
use bt_hci::controller::ExternalController;
use esp_wifi::ble::controller::BleConnector;
use crate::system::hal::radio::AsyncRadio;
// TODO: hard assuming we are using ADC1, bad. even for a driver.


pub struct RadioDriver<'a> {
    hci: ExternalController<BleConnector<'a>, 20>,
}

impl<'a> RadioDriver<'a> {
    pub fn new(hci: ExternalController<BleConnector<'a>, 20>) -> Self {
        Self { hci }
    }
}


#[async_trait(?Send)]
impl AsyncRadio for RadioDriver<'_> {

}
