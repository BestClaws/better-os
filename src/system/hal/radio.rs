use alloc::boxed::Box;

use async_trait::async_trait;
use bt_hci::controller::ExternalController;
use esp_radio::ble::controller::BleConnector;
use trouble_host::prelude::DefaultPacketPool;
use trouble_host::Stack;

#[async_trait(?Send)]
pub trait AsyncRadio {
    async fn get_stack(&mut self) -> Stack<ExternalController<BleConnector<'static>, 20>, DefaultPacketPool>;
}
