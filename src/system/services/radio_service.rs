use trouble_host::prelude::{AttributeHandle, DefaultPacketPool, FromGatt};
use bt_hci::uuid::{appearance, BluetoothUuid16};
use defmt::{info, warn};
use embassy_futures::join::join;
use embassy_time::{Duration, Timer};
use trouble_host::{Address, BleHostError, Controller, Error, Host, HostResources, PacketPool};
use trouble_host::advertise::{AdStructure, Advertisement, BR_EDR_NOT_SUPPORTED, LE_GENERAL_DISCOVERABLE};
use trouble_host::gap::{GapConfig, PeripheralConfig};
use trouble_host::gatt::{GattConnection, GattConnectionEvent, GattEvent};
use trouble_host::peripheral::Peripheral;
use trouble_host::prelude::{gatt_server, gatt_service, Runner};

const CONNECTIONS_MAX: usize = 1;
/// Max number of L2CAP channels (Signal and ATT)
const L2CAP_CHANNELS_MAX: usize = 2;


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


pub async fn run_ble_controller(
    controller: impl Controller,
)

{
    let address = Address::random([0xff, 0xff, 0xff, 0xff, 0xff, 0xff]);
    info!("[run] BLE address = {:?}", address.addr);

    let mut resources = HostResources::new();
    let stack = trouble_host::new(controller, &mut resources).set_random_address(address);
    let Host { mut peripheral, runner, .. } = stack.build();

    info!("[run] Starting BLE advertising and GATT server setup...");
    let server = GattServer::new_with_config(GapConfig::Peripheral(PeripheralConfig {
        name: "CANOPY",
        appearance: &BluetoothUuid16::new(0x01f),

    })).unwrap();

    let _ = join(
        ble_task(runner),
        async {
            loop {
                match advertise(&mut peripheral, &server).await {
                    Ok(conn) => {
                        info!("[run] Connected, spawning GATT + button tasks");
                        let _ = gatt_events_task(&server, &conn).await;
                    }
                    Err(e) => warn!("[adv] Advertising error: {:?}", defmt::Debug2Format(&e)),
                }
            }
        }
    ).await;
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
                            let _ = server.battery_service.percent.set(server, &1u16);
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
