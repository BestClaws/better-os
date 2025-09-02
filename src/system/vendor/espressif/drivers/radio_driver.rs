use crate::system::hal::radio::AsyncRadio;
use alloc::boxed::Box;
use async_trait::async_trait;
use bt_hci::controller::ExternalController;
use core::time::Duration;
use defmt::info;
use esp_hal::rng::Rng;
use esp_hal::{peripherals::BT, timer::timg::Timer};
use esp_radio::ble::controller::BleConnector;
use esp_radio::Controller;
use static_cell::StaticCell;
use trouble_host::prelude::{DefaultPacketPool, Peripheral, Runner};
use trouble_host::{peripheral, Address, Host, HostResources, Stack};
// TODO: hard assuming we are using ADC1, bad. even for a driver.

const CONNECTIONS_MAX: usize = 1;
/// Max number of L2CAP channels (Signal and ATT)
const L2CAP_CHANNELS_MAX: usize = 2;



pub struct RadioDriver {
    timer: Option<Timer<'static>>,
    bt: Option<BT<'static>>,
}


impl RadioDriver {
    pub fn new(timer: Timer<'static>, bt: BT<'static>) -> Self {
        Self {
            timer: Some(timer),
            bt: Some(bt),
        }
    }
}

#[async_trait(?Send)]
impl AsyncRadio for RadioDriver {
    async fn get_stack(&mut self)
    -> Stack<ExternalController<BleConnector<'static>, 20>, DefaultPacketPool>
    {
            let timer = self.timer.take().unwrap();
            let bt = self.bt.take().unwrap();
            esp_radio_preempt_baremetal::init(timer);

        static RADIO: StaticCell<Controller<'static>> = StaticCell::new();
        let radio = RADIO.init(esp_radio::init().unwrap());


        let connector = BleConnector::new(radio, bt);
        let controller: ExternalController<_, 20> = ExternalController::new(connector);

            let address = Address::random([0xff, 0xff, 0xff, 0xff, 0xff, 0xff]);
            info!("[run] BLE address = {:?}", address.addr.raw());
            let resources: &'static mut HostResources<
                DefaultPacketPool,
                CONNECTIONS_MAX,
                L2CAP_CHANNELS_MAX,
            > = Box::leak(Box::new(HostResources::new()));
            trouble_host::new(controller, resources).set_random_address(address)
    }
}


