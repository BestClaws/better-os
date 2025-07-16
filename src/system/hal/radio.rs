use alloc::boxed::Box;
       // v1.0.0
      // v1.0.0
use async_trait::async_trait;
use bt_hci::controller::ExternalController;
use esp_wifi::ble::controller::BleConnector;

#[async_trait(?Send)]
pub trait AsyncRadio<'a> {
    async fn get_controller(&'a mut self) -> &mut ExternalController<BleConnector<'a>, 20>;
}

