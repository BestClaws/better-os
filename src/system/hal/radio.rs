use alloc::boxed::Box;

use async_trait::async_trait;
use bt_hci::controller::ExternalController;
use esp_wifi::ble::controller::BleConnector;
use trouble_host::prelude::DefaultPacketPool;
use trouble_host::Stack;

#[async_trait(?Send)]
pub trait AsyncRadio<'a> {
    async fn get_stack(&mut self) -> Stack<'a,ExternalController<BleConnector<'a>, 20>, DefaultPacketPool>;

}
