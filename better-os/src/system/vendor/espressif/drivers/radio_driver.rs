use crate::system::hal::radio::AsyncRadio;
use alloc::boxed::Box;
use async_trait::async_trait;
use bt_hci::controller::ExternalController;
use defmt::info;
use esp_hal::peripherals::BT;
use esp_radio::ble::controller::BleConnector;
use trouble_host::prelude::{DefaultPacketPool, Peripheral, Runner};
use trouble_host::{Address, Host, HostResources, Stack};
// TODO: hard assuming we are using ADC1, bad. even for a driver.

const CONNECTIONS_MAX: usize = 1;
/// Max number of L2CAP channels (Signal and ATT)
const L2CAP_CHANNELS_MAX: usize = 2;

pub struct RadioDriver {
    bt: Option<BT<'static>>,
}

impl RadioDriver {
    pub fn new(bt: BT<'static>) -> Self {
        Self { bt: Some(bt) }
    }
}

#[async_trait(?Send)]
impl AsyncRadio for RadioDriver {
    async fn get_stack(
        &mut self,
    ) -> Stack<ExternalController<BleConnector<'static>, 20>, DefaultPacketPool> {
        let bt = self.bt.take().unwrap();

        let connector =
            BleConnector::new(bt, Default::default()).expect("failed to initialize BLE connector");
        let controller: ExternalController<_, 20> = ExternalController::new(connector);

        let address = Address::random([0xC0, 0x8f, 0x1a, 0x05, 0xe4, 0xff]);
        info!("[run] BLE address = {:?}", address.addr.raw());
        let resources: &'static mut HostResources<
            DefaultPacketPool,
            CONNECTIONS_MAX,
            L2CAP_CHANNELS_MAX,
        > = Box::leak(Box::new(HostResources::new()));
        trouble_host::new(controller, resources).set_random_address(address)
    }
}
