use alloc::boxed::Box;
use trouble_host::prelude::*;
use bt_hci::uuid::{appearance, BluetoothUuid16};
use defmt::{info, warn};
use embassy_futures::join::join;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::mutex::Mutex;
use embassy_time::{Duration, Timer};
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




// GATT Server definition: HID Service
#[gatt_server]
struct GattServer {
    battery_service: BatteryService,
    vibration_service: VibrationService
}

#[gatt_service(uuid = BluetoothUuid16::new(0x01))]
struct BatteryService {
    #[descriptor(uuid = BluetoothUuid16::new(0x2901), read, value = "Battery Percent")]

    #[characteristic(uuid = BluetoothUuid16::new(0x02), read)]
    percent: u16,
}

#[gatt_service(uuid = BluetoothUuid16::new(0x03))]
struct VibrationService {
    #[descriptor(uuid = BluetoothUuid16::new(0x2901), read, value = "Vibration With Duration")]
    #[characteristic(uuid = BluetoothUuid16::new(0x04), write)]
    vibrate_with_duration: u32,
    #[descriptor(uuid = BluetoothUuid16::new(0x2901), read, value = "Vibration Loop Period")]
    #[characteristic(uuid = BluetoothUuid16::new(0x05), write)]
    vibration_loop_period: u32,
}





async fn ble_task<C: Controller, P: PacketPool>(mut runner: Runner<'_, C, P>) {
    loop {
        match runner.run().await {
            Ok(_) => info!("[ble] Runner cycle completed"),
            Err(e) => warn!("[ble] Error in BLE runner: {:?}", defmt::Debug2Format(&e)),
        }
    }
}



async fn advertise<'a, 'b, C: Controller>(
    peripheral: &mut Peripheral<'a, C, DefaultPacketPool>,
    server: &'b GattServer<'_>
) -> Result<GattConnection<'a, 'b, DefaultPacketPool>, BleHostError<C::Error>> {
    let mut advertiser_data = [0; 31];
    let len = AdStructure::encode_slice(
        &[
            AdStructure::Flags(LE_GENERAL_DISCOVERABLE | BR_EDR_NOT_SUPPORTED),
            AdStructure::ServiceUuids16(&[[0x0f, 0x18]]),
            AdStructure::CompleteLocalName("CANOPY".as_bytes()),
        ],
        &mut advertiser_data[..],
    )?;
    let advertiser = peripheral.advertise(
        &Default::default(),
        Advertisement::ConnectableScannableUndirected {
            adv_data: &advertiser_data[..len],
            scan_data: &[],
        },
    )
        .await?;
    info!("[adv] advertising");
    let conn = advertiser.accept().await?.with_attribute_server(server)?;
    info!("[adv] connection established");
    Ok(conn)
}

async fn gatt_events_task<P: PacketPool>(
    server: &GattServer<'_>,
    conn: &GattConnection<'_, '_, P>,

) -> Result<(), Error> {
    loop {
        match conn.next().await {

            GattConnectionEvent::PhyUpdated { tx_phy, rx_phy } => {
                info!("[gatt] Phy updated. Tx phy: {}, Rx phy: {}", tx_phy, rx_phy);
            }

            GattConnectionEvent::Disconnected { reason } => {
                info!("[gatt] Disconnection due to {:?}", defmt::Debug2Format(&reason));
                Timer::after(Duration::from_millis(1000)).await;
                break Ok(())
            }
            GattConnectionEvent::Gatt { event } =>  {

                match &event {
                    GattEvent::Read(evt) => {
                        if server.battery_service.percent.handle == evt.handle() {
                            let _ = server.battery_service.percent.set(server, &99);
                        } else {
                            info!("unprocessed gatt read event: {}", defmt::Debug2Format(&evt.payload().handle()));

                        }
                    }
                    GattEvent::Write(write) => {
                        let val: u64 = write.data().iter().map(|&byte| byte as u64).sum();
                        if write.handle() == server.vibration_service.vibration_loop_period.handle() {
                            info!("[gatt] Updated vibration loop period to {} ms", val * 1000);
                        } else if write.handle() == server.vibration_service.vibrate_with_duration.handle() {
                        }
                    }

                    GattEvent::Other(_) => {}
                };

                let result = event.accept();
                match result {
                    Ok(reply) => {
                        info!("[gatt] Sending read's GATT response");
                        reply.send().await;
                    }
                    Err(e) => warn!("[gatt] Error sending read's response: {:?}", defmt::Debug2Format(&e)),
                }

            },

            GattConnectionEvent::ConnectionParamsUpdated { conn_interval, peripheral_latency, supervision_timeout } => {
                info!(
                    "[gatt] Connection parameters updated. Conn interval(ms): {}, Peripheral latency: {}, Supervision timeout(ms): {}",
                    conn_interval.as_millis(),
                    peripheral_latency,
                    supervision_timeout.as_millis()
                );
            }

        }
    }
}


#[embassy_executor::task]
pub(crate) async fn radio_service(radio: &'static Mutex<CriticalSectionRawMutex, Box<dyn AsyncRadio>>) {



    let mut radi = radio.lock().await;
    let  stack = radi.get_stack().await;

    let Host {
        mut peripheral,
        runner,
        ..
    } = stack.build();




}





