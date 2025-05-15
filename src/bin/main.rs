#![no_std]
#![no_main]

use bt_hci::controller::ExternalController;
use defmt::{info, warn};
use embassy_executor::Spawner;
use embassy_futures::join::join;
use embassy_time::{Duration, Timer};
use esp_hal::clock::CpuClock;
use esp_hal::gpio::{Input, InputConfig, Io, Level, Output, OutputConfig, Pull};
use esp_hal::timer::systimer::SystemTimer;
use esp_hal::timer::timg::TimerGroup;
use esp_wifi::ble::controller::BleConnector;
use trouble_host::{prelude::{AdStructure, Advertisement, AdvertisementParameters, DefaultPacketPool, BR_EDR_NOT_SUPPORTED, LE_GENERAL_DISCOVERABLE}, Address, Host, HostResources};
use panic_rtt_target as _;


use embassy_futures::select::select;
use trouble_host::prelude::*;

/// Max number of connections
const CONNECTIONS_MAX: usize = 1;

/// Max number of L2CAP channels.
const L2CAP_CHANNELS_MAX: usize = 2; // Signal + att


// USB HID Usage IDs for F7, F8, F9 keys
const KEY_F7: u8 = 0x40;
const KEY_F8: u8 = 0x41;
const KEY_F9: u8 = 0x42;


// GATT Server definition
#[gatt_server]
struct Server {
    hid_service: HidService,
}

/// Battery service
#[gatt_service(uuid = service::HUMAN_INTERFACE_DEVICE)]
struct HidService {
    /// Battery Level
    #[descriptor(uuid = descriptors::VALID_RANGE, read, value = [0, 100])]
    #[descriptor(uuid = descriptors::MEASUREMENT_DESCRIPTION, name = "hello", read, value = "Battery Level")]
    #[characteristic(uuid = characteristic::BATTERY_LEVEL, read, notify, value = 10)]
    level: u8,
    #[characteristic(uuid = "408813df-5dd4-1f87-ec11-cdb001100000", write, read, notify)]
    status: bool,
    
    
    // [0x01, 0x11, 0x00, 0x03] -> 1.11 version, country 0, flags 3 (remote wake, normally connectable)
    #[characteristic(uuid = BluetoothUuid16::new(0x2A4A), read, value = [0x01, 0x11, 0x00, 0x03])]
    information: [u8; 4],  

    #[characteristic(uuid = BluetoothUuid16::new(0x2A4B), read, value = [
        0x05, 0x01,       // Usage Page (Generic Desktop)
        0x09, 0x06,       // Usage (Keyboard)
        0xA1, 0x01,       // Collection (Application)
        0x75, 0x01,       //   Report Size (1)
        0x95, 0x08,       //   Report Count (8)
        0x05, 0x07,       //   Usage Page (Key Codes)
        0x19, 0xE0,       //   Usage Minimum (224)
        0x29, 0xE7,       //   Usage Maximum (231)
        0x15, 0x00,       //   Logical Minimum (0)
        0x25, 0x01,       //   Logical Maximum (1)
        0x81, 0x02,       //   Input (Data, Variable, Absolute) ; Modifier byte
        0x95, 0x01,       //   Report Count (1)
        0x75, 0x08,       //   Report Size (8)
        0x81, 0x01,       //   Input (Constant) ; Reserved byte
        0x95, 0x06,       //   Report Count (6)
        0x75, 0x08,       //   Report Size (8)
        0x15, 0x00,       //   Logical Minimum (0)
        0x25, 0x65,       //   Logical Maximum (101)
        0x05, 0x07,       //   Usage Page (Key Codes)
        0x19, 0x00,       //   Usage Minimum (0)
        0x29, 0x65,       //   Usage Maximum (101)
        0x81, 0x00,       //   Input (Data, Array)
        0xC0              // End Collection
    ])]  // Report Map
    report_map: [u8; 45],

    #[characteristic(uuid = BluetoothUuid16::new(0x2A4D), read, write, notify, value = [0, 0, 0, 0, 0, 0, 0, 0])]  // Report (Input)
    input_report: [u8; 8],  // Boot keyboard report format

    #[characteristic(uuid = BluetoothUuid16::new(0x2A4E), read, write, value = 0x00)]  // Protocol Mode
    protocol_mode: u8,  // 0x00 for Boot Protocol Mode

    #[characteristic(uuid = BluetoothUuid16::new(0x2A22), read, value = [0, 0, 0, 0, 0, 0, 0, 0])]  // Boot Keyboard Input Report
    boot_keyboard_input: [u8; 8],  // Boot keyboard format
}




extern crate alloc;



#[esp_hal_embassy::main]
async fn main(spawner: Spawner) {
    // generator version: 0.3.1
    rtt_target::rtt_init_defmt!();
    
    
    

    let config = esp_hal::Config::default().with_cpu_clock(CpuClock::max());
    let peripherals = esp_hal::init(config);
    

    esp_alloc::heap_allocator!(size: 72 * 1024);
    
    let timer0 = SystemTimer::new(peripherals.SYSTIMER);
    esp_hal_embassy::init(timer0.alarm0);
    info!("Embassy initialized!");

    let timer1 = TimerGroup::new(peripherals.TIMG0);
    let init = esp_wifi::init(
        timer1.timer0,
        esp_hal::rng::Rng::new(peripherals.RNG),
        peripherals.RADIO_CLK,
    )
    .unwrap();

    let connector = BleConnector::new(&init, peripherals.BT);
    let controller: ExternalController<_, 20> = ExternalController::new(connector);

    // // TODO: Spawn some tasks
    // let _ = spawner;
    // 
    // let address: Address = Address::random([0xff, 0x8f, 0x1a, 0x05, 0xe4, 0xff]);
    // info!("Our address");
    // 
    // let mut resources: HostResources<DefaultPacketPool, 0, 0> = HostResources::new();
    // let stack = trouble_host::new(controller, &mut resources).set_random_address(address);
    // let Host {
    //     mut peripheral,
    //     mut runner,
    //     ..
    // } = stack.build();
    // 
    // let mut adv_data = [0; 31];
    // let len = AdStructure::encode_slice(
    //     &[
    //         AdStructure::CompleteLocalName(b"Trouble Advert"),
    //         AdStructure::Flags(LE_GENERAL_DISCOVERABLE | BR_EDR_NOT_SUPPORTED),
    //     ],
    //     &mut adv_data[..],
    // )
    // .unwrap();


    let io = Io::new(peripherals.IO_MUX);
    let mut led = Output::new(peripherals.GPIO0, Level::Low, OutputConfig::default());
    let mut b1 = Input::new(peripherals.GPIO2, InputConfig::default().with_pull(Pull::Up));
    let mut b2 = Input::new(peripherals.GPIO3, InputConfig::default().with_pull(Pull::Up));
    let mut b3 = Input::new(peripherals.GPIO4, InputConfig::default().with_pull(Pull::Up));


    // info!("Starting advertising");
    // let _ = join(runner.run(), async {
    //     loop {
    //         let mut params = AdvertisementParameters::default();
    //         params.interval_min = Duration::from_millis(20);
    //         params.interval_max = Duration::from_millis(20);
    //         let _advertiser = peripheral
    //             .advertise(
    //                 &params,
    //                 Advertisement::NonconnectableScannableUndirected {
    //                     adv_data: &adv_data[..len],
    //                     scan_data: &[],
    //                 },
    //             )
    //             .await
    //             .unwrap();
    //         loop {
    //             info!("Still running");
    //             b1.wait_for_low().await;
    //             led.set_high();
    //             b2.wait_for_low().await;
    //             b3.wait_for_low().await;
    //             led.set_low();
    //         }
    //     }
    // })
    // .await;

    run(controller).await;

    // for inspiration have a look at the examples at https://github.com/esp-rs/esp-hal/tree/esp-hal-v1.0.0-beta.0/examples/src/bin
}




/// Run the BLE stack.
pub async fn run<C>(controller: C)
where
    C: Controller,
{
    // Using a fixed "random" address can be useful for testing. In real scenarios, one would
    // use e.g. the MAC 6 byte array as the address (how to get that varies by the platform).
    let address: Address = Address::random([0xff, 0x8f, 0x1a, 0x05, 0xe4, 0xff]);
    info!("Our address = {:?}", address.addr);

    let mut resources: HostResources<DefaultPacketPool, CONNECTIONS_MAX, L2CAP_CHANNELS_MAX> = HostResources::new();
    let stack = trouble_host::new(controller, &mut resources).set_random_address(address);
    let Host {
        mut peripheral, runner, ..
    } = stack.build();

    info!("Starting advertising and GATT service");
    let server = Server::new_with_config(GapConfig::Peripheral(PeripheralConfig {
        name: "TrouBLE",
        appearance: &appearance::power_device::GENERIC_POWER_DEVICE,
    }))
        .unwrap();

    let _ = join(ble_task(runner), async {
        loop {
            match advertise("Trouble Example", &mut peripheral, &server).await {
                Ok(conn) => {
                    // set up tasks when the connection is established to a central, so they don't run when no one is connected.
                    let a = gatt_events_task(&server, &conn);
                    let b = custom_task(&server, &conn, &stack);
                    // run until any task ends (usually because the connection has been closed),
                    // then return to advertising state.
                    select(a, b).await;
                }
                Err(e) => {
                    let e = defmt::Debug2Format(&e);
                    panic!("[adv] error: {:?}", e);
                }
            }
        }
    })
        .await;
}

/// This is a background task that is required to run forever alongside any other BLE tasks.
///
/// ## Alternative
///
/// If you didn't require this to be generic for your application, you could statically spawn this with i.e.
///
/// ```rust,ignore
///
/// #[embassy_executor::task]
/// async fn ble_task(mut runner: Runner<'static, SoftdeviceController<'static>>) {
///     runner.run().await;
/// }
///
/// spawner.must_spawn(ble_task(runner));
/// ```
async fn ble_task<C: Controller, P: PacketPool>(mut runner: Runner<'_, C, P>) {
    loop {
        if let Err(e) = runner.run().await {
            let e = defmt::Debug2Format(&e);
            panic!("[ble_task] error: {:?}", e);
        }
    }
}

/// Stream Events until the connection closes.
///
/// This function will handle the GATT events and process them.
/// This is how we interact with read and write requests.
async fn gatt_events_task<P: PacketPool>(server: &Server<'_>, conn: &GattConnection<'_, '_, P>) -> Result<(), Error> {
    let level = server.hid_service.level;
    loop {
        match conn.next().await {
            GattConnectionEvent::Disconnected { reason } => {
                info!("[gatt] disconnected: {:?}", reason);
                break;
            }
            GattConnectionEvent::Gatt { event } => match event {
                Ok(event) => {
                    match &event {
                        GattEvent::Read(event) => {
                            if event.handle() == level.handle {
                                let value = server.get(&level);
                                info!("[gatt] Read Event to Level Characteristic: {:?}", defmt::Debug2Format(&value));
                            }
                        }
                        GattEvent::Write(event) => {
                            if event.handle() == level.handle {
                                info!("[gatt] Write Event to Level Characteristic: {:?}", event.data());
                            }
                        }
                    }

                    // This step is also performed at drop(), but writing it explicitly is necessary
                    // in order to ensure reply is sent.
                    match event.accept() {
                        Ok(reply) => {
                            reply.send().await;
                        }
                        Err(e) => warn!("[gatt] error sending response: {:?}", defmt::Debug2Format(&e)),
                    }
                }
                Err(e) => warn!("[gatt] error processing event: {:?}", defmt::Debug2Format(&e)),
            },
            _ => {}
        }
    }
    info!("[gatt] task finished");
    Ok(())
}

/// Create an advertiser to use to connect to a BLE Central, and wait for it to connect.
async fn advertise<'a, 'b, C: Controller>(
    name: &'a str,
    peripheral: &mut Peripheral<'a, C, DefaultPacketPool>,
    server: &'b Server<'_>,
) -> Result<GattConnection<'a, 'b, DefaultPacketPool>, BleHostError<C::Error>> {
    let mut advertiser_data = [0; 31];
    let len = AdStructure::encode_slice(
        &[
            AdStructure::Flags(LE_GENERAL_DISCOVERABLE | BR_EDR_NOT_SUPPORTED),
            AdStructure::ServiceUuids16(&[[0x0f, 0x18]]),
            AdStructure::CompleteLocalName(name.as_bytes()),
        ],
        &mut advertiser_data[..],
    )?;
    let advertiser = peripheral
        .advertise(
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

/// Example task to use the BLE notifier interface.
/// This task will notify the connected central of a counter value every 2 seconds.
/// It will also read the RSSI value every 2 seconds.
/// and will stop when the connection is closed by the central or an error occurs.
async fn custom_task<C: Controller, P: PacketPool>(
    server: &Server<'_>,
    conn: &GattConnection<'_, '_, P>,
    stack: &Stack<'_, C, P>,
) {
    let mut tick: u8 = 0;
    let level = server.hid_service.level;
    loop {
        tick = tick.wrapping_add(1);
        info!("[custom_task] notifying connection of tick {}", tick);
        if level.notify(conn, &tick).await.is_err() {
            info!("[custom_task] error notifying connection");
            break;
        };
        // read RSSI (Received Signal Strength Indicator) of the connection.
        if let Ok(rssi) = conn.raw().rssi(stack).await {
            info!("[custom_task] RSSI: {:?}", rssi);
        } else {
            info!("[custom_task] error getting RSSI");
            break;
        };
        Timer::after_secs(2).await;
    }
}
