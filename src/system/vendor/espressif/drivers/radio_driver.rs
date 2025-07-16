use alloc::boxed::Box;
use core::time::Duration;
use async_trait::async_trait;
use bt_hci::controller::ExternalController;
use defmt::info;
use esp_wifi::ble::controller::BleConnector;
use esp_wifi::EspWifiController;
use trouble_host::prelude::{DefaultPacketPool, Peripheral, Runner};
use crate::system::hal::radio::AsyncRadio;
use esp_hal::peripherals::BT;
use trouble_host::{peripheral, Address, Host, HostResources, Stack};
// TODO: hard assuming we are using ADC1, bad. even for a driver.

const CONNECTIONS_MAX: usize = 1;
/// Max number of L2CAP channels (Signal and ATT)
const L2CAP_CHANNELS_MAX: usize = 2;


pub struct RadioDriver<'a> {
    stack:  Option<Stack<'a,ExternalController<BleConnector<'a>, 20>, DefaultPacketPool>>
}

impl<'a> RadioDriver<'a> {
    pub fn new(radio_init: EspWifiController<'a>, bt: BT<'a>) -> Self {
        let init = Box::leak(Box::new(radio_init));
        let connector = BleConnector::new(init, bt);
        let controller: ExternalController<_, 20> = ExternalController::new(connector);
        let address = Address::random([0xff, 0xff, 0xff, 0xff, 0xff, 0xff]);
        info!("[run] BLE address = {:?}", address.addr);
        let  resources: &'static mut HostResources<DefaultPacketPool, CONNECTIONS_MAX, L2CAP_CHANNELS_MAX>
            = Box::leak(Box::new(HostResources::new()));
        let stack = trouble_host::new(controller, resources).set_random_address(address);
        Self { stack: Some(stack) }
    }
}


#[async_trait(?Send)]
impl<'a> AsyncRadio<'a> for RadioDriver<'a> {
    async fn get_stack(&mut self) -> Stack<'a,ExternalController<BleConnector<'a>, 20>, DefaultPacketPool> {
        self.stack.take().unwrap()
    }
}
