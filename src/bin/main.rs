//! BLE HID Keyboard example on ESP32 (three buttons → F7, F8, F9)
#![no_std]
#![no_main]

extern crate alloc;
use bt_hci::controller::ExternalController;
use bt_hci::param::{DisconnectReason, Status};
use defmt::{info, warn};
use embassy_executor::Spawner;
use embassy_futures::select::Either;
use embassy_futures::{join::join, select::select};
use embassy_time::{Duration, Timer};
use esp_hal::clock::CpuClock;
use esp_hal::gpio::{Input, InputConfig, Io, Level, Output, OutputConfig, Pull};
use esp_hal::timer::systimer::SystemTimer;
use esp_hal::timer::timg::TimerGroup;
use esp_wifi::ble::controller::BleConnector;
use panic_rtt_target as _;
use trouble_host::{prelude::*, Address, Host, HostResources};
use rand_core::{CryptoRng, RngCore};

/// Max number of connections
const CONNECTIONS_MAX: usize = 1;
/// Max number of L2CAP channels (Signal + ATT)
const L2CAP_CHANNELS_MAX: usize = 2;

/// USB HID Usage IDs for F7, F8, F9
const KEY_F7: u8 = 0x40;
const KEY_F8: u8 = 0x41;
const KEY_F9: u8 = 0x42;

const KEY_A: u8 = 0x04;
const KEY_B: u8 = 0x05;
const KEY_C: u8 = 0x06;


// GATT Server definition: HID Service
#[gatt_server]
struct Server {
    hid: HidService,
}

// #[gatt_service(uuid = "12345678-1234-5678-1234-56789abcdef0")]
#[gatt_service(uuid = service::HUMAN_INTERFACE_DEVICE)]
struct HidService {

    // #[characteristic(uuid = "12345678-1234-5678-1234-56789abcdef1", read, value = [0x01, 0x11, 0x00, 0x03])]
    #[characteristic(uuid = BluetoothUuid16::new(0x2A4A), read, value = [0x01, 0x11, 0x00, 0x03])]
    information: [u8; 4],

    // #[characteristic(uuid = "12345678-1234-5678-1234-56789abcdef2", read, value = [
    #[characteristic(uuid = BluetoothUuid16::new(0x2A4B), read, value = [
    0x05, 0x01, 0x09, 0x06, 0xA1, 0x01, 0x75, 0x01, 0x95, 0x08,
    0x05, 0x07, 0x19, 0xE0, 0x29, 0xE7, 0x15, 0x00, 0x25, 0x01,
    0x81, 0x02, 0x95, 0x01, 0x75, 0x08, 0x81, 0x01, 0x95, 0x06,
    0x75, 0x08, 0x15, 0x00, 0x25, 0x65, 0x05, 0x07, 0x19, 0x00,
    0x29, 0x65, 0x81, 0x00, 0xC0
    ])]
    report_map: [u8; 45],
    // #[characteristic(uuid = "12345678-1234-5678-1234-56789abcdef3", read, notify, value = [0; 8])]

    #[characteristic(uuid = BluetoothUuid16::new(0x2A4D), read, notify, write, value = [0; 8])]
    input_report: [u8; 8],

    // #[characteristic(uuid = "12345678-1234-5678-1234-56789abcdef4", read, write, value = 0x00)]
    #[characteristic(uuid = BluetoothUuid16::new(0x2A4E), read, write, value = 0x00)]
    protocol_mode: u8,

    // #[characteristic(uuid = "12345678-1234-5678-1234-56789abcdef5", read, value = [0; 8])]
    #[characteristic(uuid = BluetoothUuid16::new(0x2A22), read, value = [0; 8])]
    boot_keyboard_input: [u8; 8],
}

#[esp_hal_embassy::main]
async fn main(spawner: Spawner) {
    rtt_target::rtt_init_defmt!();

    let config = esp_hal::Config::default().with_cpu_clock(CpuClock::max());
    let peripherals = esp_hal::init(config);
    esp_alloc::heap_allocator!(size: 72 * 1024);

    let timer0 = SystemTimer::new(peripherals.SYSTIMER);
    esp_hal_embassy::init(timer0.alarm0);
    info!("[main] Embassy initialized");

    let mut rng = esp_hal::rng::Trng::new(peripherals.RNG, peripherals.ADC1);

    let timer1 = TimerGroup::new(peripherals.TIMG0);
    let init = esp_wifi::init(
        timer1.timer0,
        rng.rng.clone(),
        peripherals.RADIO_CLK,
    ).unwrap();

    let connector = BleConnector::new(&init, peripherals.BT);
    let controller: ExternalController<_, 20> = ExternalController::new(connector);

    let io = Io::new(peripherals.IO_MUX);
    let mut led = Output::new(peripherals.GPIO0, Level::Low, OutputConfig::default());
    let mut b1 = Input::new(peripherals.GPIO2, InputConfig::default().with_pull(Pull::Up));
    let mut b2 = Input::new(peripherals.GPIO3, InputConfig::default().with_pull(Pull::Up));
    let mut b3 = Input::new(peripherals.GPIO4, InputConfig::default().with_pull(Pull::Up));



    spawner.spawn(working()).unwrap();

    run(controller, &mut b1, &mut b2, &mut b3, &mut led, &mut rng).await;
}

#[embassy_executor::task]
async fn working() {
    loop {
        info!("{:?}", embassy_time::Instant::now().as_secs());
        embassy_time::Timer::after_secs(1).await; 
    }
}

async fn run<C, RNG>(
    controller: C,
    b1: &mut Input<'_>,
    b2: &mut Input<'_>,
    b3: &mut Input<'_>,
    led: &mut Output<'_>,
    random_generator: &mut RNG)
where
    C: Controller,
    RNG: RngCore + CryptoRng,
{
    let address = Address::random([0xff, 0x8f, 0x1a, 0x05, 0xe4, 0xff]);
    info!("[run] Our BLE address = {:?}", address.addr);

    let mut resources: HostResources<DefaultPacketPool, CONNECTIONS_MAX, L2CAP_CHANNELS_MAX> = HostResources::new();
    let stack = trouble_host::new(controller, &mut resources).set_random_address(address).set_random_generator_seed(random_generator);
    let Host { mut peripheral, runner, .. } = stack.build();

    info!("[run] Starting BLE advertising and GATT server setup...");
    let server = Server::new_with_config(GapConfig::Peripheral(PeripheralConfig {
        name: "ESP32-HID",
        appearance: &appearance::human_interface_device::KEYBOARD,

    })).unwrap();

    let _ = join(
        ble_task(runner),
        async {
            loop {
                match advertise_hid(&mut peripheral, &server).await {
                    Ok(conn) => {
                        info!("[run] Connected, spawning GATT + button tasks");
                        let g = gatt_events_task(&server, &conn);
                        let b = button_task(&server, &conn, b1, b2, b3, led);
                        select(g, b).await;
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



async fn advertise_hid<'a, 'b, C: Controller>(
    peripheral: &mut Peripheral<'a, C, DefaultPacketPool>,
    server: &'b Server<'_>
) -> Result<GattConnection<'a, 'b, DefaultPacketPool>, BleHostError<C::Error>> {
    let mut adv_data = [0; 31];
    const PNP_ID: [u8; 7] = [
        0x01,       // Vendor ID source: Bluetooth SIG
        0x34, 0x12, // Vendor ID (0x1234 little-endian)
        0x78, 0x56, // Product ID (0x5678 little-endian)
        0x00, 0x01, // Product Version (0x0100 little-endian)
    ];


    let len = AdStructure::encode_slice(
        &[
            AdStructure::Flags(LE_GENERAL_DISCOVERABLE | BR_EDR_NOT_SUPPORTED),
            AdStructure::ServiceUuids16(&[[0x12, 0x18]]),
            AdStructure::CompleteLocalName(b"HID"),
            AdStructure::ManufacturerSpecificData {
                company_identifier: 0x004C,
                payload: &[0x01, 0x02, 0x03],
            },
        ],
        &mut adv_data,
    )?;

    let adv = peripheral
        .advertise(&AdvertisementParameters {
            ..AdvertisementParameters::default()
        },
                   Advertisement::ConnectableScannableUndirected {
                       adv_data: &adv_data[..len],
                       scan_data: &[],
                   }
        )
        .await?;
    info!("[adv] BLE advertising started");
    let conn = adv.accept().await?.with_attribute_server(server)?;
    info!("[adv] BLE connection established");
    Ok(conn)
}

async fn gatt_events_task<P: PacketPool>(
    server: &Server<'_>,
    conn: &GattConnection<'_, '_, P>,
) -> Result<(), Error> {
    loop {
        let hid_handle = server.hid.handle;
        let hid_information_handle = server.hid.information.handle;
        let report_handle = server.hid.report_map.handle;
        let input_handle = server.hid.input_report.handle;
        let protocol_handle = server.hid.protocol_mode.handle;
        let boot_handle = server.hid.boot_keyboard_input.handle;

        match conn.next().await {

            GattConnectionEvent::Bonded {
                bond_info
            } => {

            }
            GattConnectionEvent::PhyUpdated { tx_phy, rx_phy } => {
                info!("[gatt] Phy updated. Tx phy: {}, Rx phy: {}", tx_phy, rx_phy);
            }

            GattConnectionEvent::Disconnected { reason } => {
                info!("[gatt] Disconnected: {:?}", reason);
                match reason {
                    _ => {
                        info!("[gatt] Disconnection due to Unknown HCI Command");
                    }
                    Status::UNKNOWN_CONN_IDENTIFIER => {
                        info!("[gatt] Disconnection due to Unknown Connection Identifier");
                    }
                    Status::HARDWARE_FAILURE => {
                        info!("[gatt] Disconnection due to Hardware Failure");
                    }
                    Status::PAGE_TIMEOUT => {
                        info!("[gatt] Disconnection due to Page Timeout");
                    }
                    Status::AUTHENTICATION_FAILURE => {
                        info!("[gatt] Disconnection due to Authentication Failure");
                    }
                    Status::PIN_OR_KEY_MISSING => {
                        info!("[gatt] Disconnection due to PIN or Key Missing");
                    }
                    Status::MEMORY_CAPACITY_EXCEEDED => {
                        info!("[gatt] Disconnection due to Memory Capacity Exceeded");
                    }
                    Status::CONN_TIMEOUT => {
                        info!("[gatt] Disconnection due to Connection Timeout");
                    }
                    Status::CONN_LIMIT_EXCEEDED => {
                        info!("[gatt] Disconnection due to Connection Limit Exceeded");
                    }
                    Status::SYNCHRONOUS_CONN_LIMIT_EXCEEDED => {
                        info!("[gatt] Disconnection due to Synchronous Connection Limit Exceeded");
                    }
                    Status::CONN_ALREADY_EXISTS => {
                        info!("[gatt] Disconnection due to Connection Already Exists");
                    }
                    Status::CMD_DISALLOWED => {
                        info!("[gatt] Disconnection due to Command Disallowed");
                    }
                    Status::CONN_REJECTED_LIMITED_RESOURCES => {
                        info!("[gatt] Disconnection due to Connection Rejected (Limited Resources)");
                    }
                    Status::CONN_REJECTED_SECURITY_REASONS => {
                        info!("[gatt] Disconnection due to Connection Rejected (Security Reasons)");
                    }
                    Status::CONN_REJECTED_UNACCEPTABLE_BD_ADDR => {
                        info!("[gatt] Disconnection due to Connection Rejected (Unacceptable BD_ADDR)");
                    }
                    Status::CONN_ACCEPT_TIMEOUT_EXCEEDED => {
                        info!("[gatt] Disconnection due to Connection Accept Timeout Exceeded");
                    }
                    Status::UNSUPPORTED => {
                        info!("[gatt] Disconnection due to Unsupported Feature or Parameter Value");
                    }
                    Status::INVALID_HCI_PARAMETERS => {
                        info!("[gatt] Disconnection due to Invalid HCI Command Parameters");
                    }
                    Status::REMOTE_USER_TERMINATED_CONN => {
                        info!("[gatt] Disconnection due to Remote User Terminated Connection");
                    }
                    Status::REMOTE_DEVICE_TERMINATED_CONN_LOW_RESOURCES => {
                        info!("[gatt] Disconnection due to Remote Device Terminated (Low Resources)");
                    }
                    Status::REMOTE_DEVICE_TERMINATED_CONN_POWER_OFF => {
                        info!("[gatt] Disconnection due to Remote Device Terminated (Power Off)");
                    }
                    Status::CONN_TERMINATED_BY_LOCAL_HOST => {
                        info!("[gatt] Disconnection due to Connection Terminated by Local Host");
                    }
                    Status::REPEATED_ATTEMPTS => {
                        info!("[gatt] Disconnection due to Repeated Attempts");
                    }
                    Status::PAIRING_NOT_ALLOWED => {
                        info!("[gatt] Disconnection due to Pairing Not Allowed");
                    }
                    Status::UNKNOWN_LMP_PDU => {
                        info!("[gatt] Disconnection due to Unknown LMP PDU");
                    }
                    Status::UNSUPPORTED_REMOTE_FEATURE => {
                        info!("[gatt] Disconnection due to Unsupported Remote Feature");
                    }
                    Status::SCO_OFFSET_REJECTED => {
                        info!("[gatt] Disconnection due to SCO Offset Rejected");
                    }
                    Status::SCO_INTERVAL_REJECTED => {
                        info!("[gatt] Disconnection due to SCO Interval Rejected");
                    }
                    Status::SCO_AIR_MODE_REJECTED => {
                        info!("[gatt] Disconnection due to SCO Air Mode Rejected");
                    }
                    Status::INVALID_LMP_LL_PARAMETERS => {
                        info!("[gatt] Disconnection due to Invalid LMP/LL Parameters");
                    }
                    Status::UNSPECIFIED => {
                        info!("[gatt] Disconnection due to Unspecified Error");
                    }
                    Status::UNSUPPORTED_LMP_LL_PARAMETER_VALUE => {
                        info!("[gatt] Disconnection due to Unsupported LMP/LL Parameter Value");
                    }
                    Status::ROLE_CHANGE_NOT_ALLOWED => {
                        info!("[gatt] Disconnection due to Role Change Not Allowed");
                    }
                    Status::LMP_LL_RESPONSE_TIMEOUT => {
                        info!("[gatt] Disconnection due to LMP/LL Response Timeout");
                    }
                    Status::LMP_LL_COLLISION => {
                        info!("[gatt] Disconnection due to LMP/LL Procedure Collision");
                    }
                    Status::LMP_PDU_NOT_ALLOWED => {
                        info!("[gatt] Disconnection due to LMP PDU Not Allowed");
                    }
                    Status::ENCRYPTION_MODE_NOT_ACCEPTABLE => {
                        info!("[gatt] Disconnection due to Encryption Mode Not Acceptable");
                    }
                    Status::LINK_KEY_CANNOT_BE_CHANGED => {
                        info!("[gatt] Disconnection due to Link Key Cannot Be Changed");
                    }
                    Status::REQUESTED_QOS_NOT_SUPPORTED => {
                        info!("[gatt] Disconnection due to Requested QoS Not Supported");
                    }
                    Status::INSTANT_PASSED => {
                        info!("[gatt] Disconnection due to Instant Passed");
                    }
                    Status::PAIRING_WITH_UNIT_KEY_NOT_SUPPORTED => {
                        info!("[gatt] Disconnection due to Pairing With Unit Key Not Supported");
                    }
                    Status::DIFFERENT_TRANSACTION_COLLISION => {
                        info!("[gatt] Disconnection due to Different Transaction Collision");
                    }
                    Status::QOS_UNACCEPTABLE_PARAMETER => {
                        info!("[gatt] Disconnection due to QoS Unacceptable Parameter");
                    }
                    Status::QOS_REJECTED => {
                        info!("[gatt] Disconnection due to QoS Rejected");
                    }
                    Status::CHANNEL_CLASSIFICATION_NOT_SUPPORTED => {
                        info!("[gatt] Disconnection due to Channel Classification Not Supported");
                    }
                    Status::INSUFFICIENT_SECURITY => {
                        info!("[gatt] Disconnection due to Insufficient Security");
                    }
                    Status::PARAMETER_OUT_OF_RANGE => {
                        info!("[gatt] Disconnection due to Parameter Out Of Mandatory Range");
                    }
                    Status::ROLE_SWITCH_PENDING => {
                        info!("[gatt] Disconnection due to Role Switch Pending");
                    }
                    Status::RESERVED_SLOT_VIOLATION => {
                        info!("[gatt] Disconnection due to Reserved Slot Violation");
                    }
                    Status::ROLE_SWITCH_FAILED => {
                        info!("[gatt] Disconnection due to Role Switch Failed");
                    }
                    Status::EXT_INQUIRY_RESPONSE_TOO_LARGE => {
                        info!("[gatt] Disconnection due to Extended Inquiry Response Too Large");
                    }
                    Status::SECURE_SIMPLE_PAIRING_NOT_SUPPORTED_BY_HOST => {
                        info!("[gatt] Disconnection due to Secure Simple Pairing Not Supported By Host");
                    }
                    Status::HOST_BUSY_PAIRING => {
                        info!("[gatt] Disconnection due to Host Busy - Pairing");
                    }
                    Status::CONN_REJECTED_NO_SUITABLE_CHANNEL_FOUND => {
                        info!("[gatt] Disconnection due to Connection Rejected (No Suitable Channel)");
                    }
                    Status::CONTROLLER_BUSY => {
                        info!("[gatt] Disconnection due to Controller Busy");
                    }
                    Status::UNACCEPTABLE_CONN_PARAMETERS => {
                        info!("[gatt] Disconnection due to Unacceptable Connection Parameters");
                    }
                    Status::ADV_TIMEOUT => {
                        info!("[gatt] Disconnection due to Advertising Timeout");
                    }
                    Status::CONN_TERMINATED_DUE_TO_MIC_FAILURE => {
                        info!("[gatt] Disconnection due to Connection Terminated (MIC Failure)");
                    }
                    Status::CONN_FAILED_SYNCHRONIZATION_TIMEOUT => {
                        info!("[gatt] Disconnection due to Connection Failed (Synchronization Timeout)");
                    }
                    Status::COARSE_CLOCK_ADJUSTMENT_REJECTED => {
                        info!("[gatt] Disconnection due to Coarse Clock Adjustment Rejected");
                    }
                    Status::TYPE0_SUBMAP_NOT_DEFINED => {
                        info!("[gatt] Disconnection due to Type0 Submap Not Defined");
                    }
                    Status::UNKNOWN_ADV_IDENTIFIER => {
                        info!("[gatt] Disconnection due to Unknown Advertising Identifier");
                    }
                    Status::LIMIT_REACHED => {
                        info!("[gatt] Disconnection due to Limit Reached");
                    }
                    Status::OPERATION_CANCELLED_BY_HOST => {
                        info!("[gatt] Disconnection due to Operation Cancelled by Host");
                    }
                    Status::PACKET_TOO_LONG => {
                        info!("[gatt] Disconnection due to Packet Too Long");
                    },
                    _ => todo!()


                }
                embassy_time::Timer::after(Duration::from_millis(1000)).await;
                break Ok(())
            }
            GattConnectionEvent::Gatt { event } => match event {
                Ok(evt) => {
                    info!("[gatt] Received event");

                    let result = match &evt {
                        GattEvent::Read(r) => {



                            if r.handle() == hid_information_handle {
                                    info!("[gatt] Received read request on server_handle");
                            } else if r.handle() == report_handle {
                                info!("[gatt] Received read request on report handle");
                            } else if r.handle() == input_handle {
                                info!("[gatt] Received read request on input_handle");
                            } else if r.handle() == protocol_handle {
                                info!("[gatt] Received read request on protocol_handle");
                            } else if r.handle() == boot_handle {
                                info!("[gatt] Received read request on boot_handle");
                            } else if r.handle() == hid_handle {
                                info!("[gatt] Received read request on hid_handle");
                            } else {
                                info!("[gatt] Received read request on {}", r.handle());
                            }


                            if conn.raw().encrypted() {
                                None
                            } else {
                                Some(AttErrorCode::INSUFFICIENT_ENCRYPTION)
                            }
                        }
                        GattEvent::Write(w) => {
                            if w.handle() == hid_information_handle {
                                info!("[gatt] Received write request on server_handle: {:?}", defmt::Debug2Format(&w.data()));
                            } else if w.handle() == report_handle {
                                info!("[gatt] Received write request on report handle: {:?}", defmt::Debug2Format(&w.data()));
                            } else if w.handle() == input_handle {
                                info!("[gatt] Received write request on input_handle: {:?}", defmt::Debug2Format(&w.data()));
                            } else if w.handle() == protocol_handle {
                                info!("[gatt] Received write request on protocol_handle: {:?}", defmt::Debug2Format(&w.data()));
                            } else if w.handle() == boot_handle {
                                info!("[gatt] Received write request on boot_handle: {:?}", defmt::Debug2Format(&w.data()));
                            } else if w.handle() == hid_handle {
                                info!("[gatt] Received write request on hid_handle: {:?}", defmt::Debug2Format(&w.data()));
                            } else {
                                info!("[gatt] Received write request: {:?} on handle: {}", defmt::Debug2Format(&w.data()), w.handle());
                            }





                            if conn.raw().encrypted() {
                                None
                            } else {
                                Some(AttErrorCode::INSUFFICIENT_ENCRYPTION)
                            }
                        }
                    };

                    let result = if let Some(code) = result {
                        evt.reject(code)
                    } else {
                        evt.accept()
                    };
                    match result {
                        Ok(mut reply) => {

                            info!("[gatt] Sending read's GATT response");

                            reply.send().await;
                        }
                        Err(e) => warn!("[gatt] Error sending read's response: {:?}", defmt::Debug2Format(&e)),
                    }

                }
                Err(e) => warn!("[gatt] GATT event error: {:?}", defmt::Debug2Format(&e)),
            },

            GattConnectionEvent::ConnectionParamsUpdated { conn_interval, peripheral_latency, supervision_timeout } => {
                info!("[gatt] Connection parameters updated. Conn interval(ms): {}, Peripheral latency: {}, Supervision timeout(ms): {}", conn_interval.as_millis(), peripheral_latency, supervision_timeout.as_millis());
            }

        }
    }
}

async fn button_task<P: PacketPool>(
    server: &Server<'_>,
    conn: &GattConnection<'_, '_, P>,
    b1: &mut Input<'_>,
    b2: &mut Input<'_>,
    b3: &mut Input<'_>,
    led: &mut Output<'_>,
) {
    loop {
        info!("[btn] Waiting for key press...");

        let key = select(
            async { b1.wait_for_low().await; KEY_F7 },
            select(
                async { b2.wait_for_low().await; KEY_F8 },
                async { b3.wait_for_low().await; KEY_F9 },
            ),
        ).await;

        let mut report = [0u8; 8];

        match key {
            Either::First(x) => { report[2] = x; info!("[btn] F7 pressed"); }
            Either::Second(y) => match y {
                Either::First(z) => { report[2] = z; info!("[btn] F8 pressed"); }
                Either::Second(z2) => { report[2] = z2; info!("[btn] F9 pressed"); }
            }
        }

        info!("[hid] Sending key press report: {:?}", report);
        led.set_high();
        match server.hid.input_report.notify(conn, &report).await {
            Ok(_) => info!("[hid] Key press notification sent"),
            Err(e) => warn!("[hid] Failed to notify key press: {:?}", defmt::Debug2Format(&e)),
        }

        Timer::after(Duration::from_millis(100)).await;

        match server.hid.input_report.notify(conn, &[0; 8]).await {
            Ok(_) => info!("[hid] Key release notification sent"),
            Err(e) => warn!("[hid] Failed to notify key release: {:?}", defmt::Debug2Format(&e)),
        }

        led.set_low();
    }
}
