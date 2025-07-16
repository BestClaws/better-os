use alloc::boxed::Box;

use async_trait::async_trait;
use bt_hci::controller::ExternalController;
use esp_wifi::ble::controller::BleConnector;
use trouble_host::prelude::DefaultPacketPool;
use trouble_host::Stack;

#[async_trait(?Send)]
pub trait AsyncRadio<'a> {
    async fn get_stack(&'a mut self) -> Stack<'_,ExternalController<BleConnector<'a>, 20>, DefaultPacketPool>;

    async fn get_foo(&self) -> Bar<'_> {

        todo!() // Placeholder implementation
    }
}
struct Bar<'a> { s: &'a str}