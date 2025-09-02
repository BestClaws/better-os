use alloc::boxed::Box;
use trouble_host::prelude::*;
use bt_hci::uuid::{appearance, BluetoothUuid16};
use defmt::{info, warn};
use embassy_futures::join::join;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::mutex::Mutex;
use embassy_time::{Duration, Timer};
use static_cell::StaticCell;
use trouble_host::{peripheral, Address, BleHostError, Controller, Error, Host, HostResources, PacketPool};
use trouble_host::advertise::{AdStructure, Advertisement, BR_EDR_NOT_SUPPORTED, LE_GENERAL_DISCOVERABLE};
use trouble_host::gap::{GapConfig, PeripheralConfig};
use trouble_host::gatt::{GattConnection, GattConnectionEvent, GattEvent};
use trouble_host::peripheral::Peripheral;
use trouble_host::prelude::{gatt_server, gatt_service, AttributeHandle, DefaultPacketPool, Runner};
use crate::system::hal::imu::AsyncGyroAccelerometer;
use crate::system::hal::radio::AsyncRadio;
use crate::system::services::gyro_accel_srv::ORIENTATION_CHANNEL;
use crate::system::vendor::invensense::drivers::mpu6050::sensor::{get_gravity, get_yaw_pitch_roll};
use crate::util::math::primitives::Vec3;






#[embassy_executor::task]
pub(crate) async fn radio_service(
    radio: &'static Mutex<CriticalSectionRawMutex, Box<dyn AsyncRadio>>,
) {

    let mut radio_g = radio.lock().await;
    let stack = radio_g.get_stack().await;
    let Host { mut peripheral, mut runner, ..} = stack.build();







    let mut adv_data = [0; 31];
    let len = AdStructure::encode_slice(
        &[
            AdStructure::CompleteLocalName(b"Trouble Advert"),
            AdStructure::Flags(LE_GENERAL_DISCOVERABLE | BR_EDR_NOT_SUPPORTED),
        ],
        &mut adv_data[..],
    )
        .unwrap();

    info!("Starting advertising");

    let _ = join(runner.run(), async {
        let mut params = AdvertisementParameters::default();
        params.interval_min = Duration::from_millis(100);
        params.interval_max = Duration::from_millis(100);

        let _advertiser = peripheral
            .advertise(
                &params,
                Advertisement::NonconnectableScannableUndirected {
                    adv_data: &adv_data[..len],
                    scan_data: &[],
                },
            )
            .await
            .unwrap();

        panic!("hi");
        

        loop {
            info!("Still running");
            Timer::after(Duration::from_secs(1)).await;


        }
    })
        .await;


}





