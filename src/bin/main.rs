//! BLE HID Keyboard example on ESP32 (three buttons → F7, F8, F9)
#![no_std]
#![no_main]

extern crate alloc;
use core::cell::RefCell;
use nb;

use bt_hci::controller::ExternalController;
use bt_hci::param::{DisconnectReason, Status};
use critical_section::Mutex;
use defmt::{info, warn};
use embassy_executor::Spawner;
use embassy_futures::select::Either;
use embassy_futures::{join::join, select::select};
use embassy_time::{Duration, Timer};
use esp_hal::analog::adc::{Adc, AdcConfig, Attenuation};
use esp_hal::clock::CpuClock;
use esp_hal::gpio::{Input, InputConfig, Io, Level, Output, OutputConfig, Pull};
use esp_hal::timer::systimer::SystemTimer;
use esp_hal::timer::timg::TimerGroup;
use esp_wifi::ble::controller::BleConnector;
use panic_rtt_target as _;
use rand_core::{CryptoRng, RngCore};
use trouble_host::{prelude::*, Address, Host, HostResources};

/// Max number of connections
const CONNECTIONS_MAX: usize = 1;
/// Max number of L2CAP channels (Signal + ATT)
const L2CAP_CHANNELS_MAX: usize = 2;

static VIBRATOR: Mutex<RefCell<Option<Output>>> = Mutex::new(RefCell::new(None));
static VIBRATION_DURATION: Mutex<RefCell<Option<u32>>> = Mutex::new(RefCell::new(Some(5000))); // Default 5 seconds

// GATT Server definition: HID Service
#[gatt_server]
struct Server {
    hid: CanopyService,
}

// #[gatt_service(uuid = "12345678-1234-5678-1234-56789abcdef0")]
#[gatt_service(uuid = BluetoothUuid16::new(0xAAAA))]
struct CanopyService {
    #[characteristic(uuid = BluetoothUuid16::new(0xAAAB), read, write)]
    amount: u8,

    #[characteristic(uuid = BluetoothUuid16::new(0xAAAC), write)]
    vibration_duration: u32, // New characteristic for vibration duration
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
    let init = esp_wifi::init(timer1.timer0, rng.rng.clone(), peripherals.RADIO_CLK).unwrap();

    let connector = BleConnector::new(&init, peripherals.BT);
    let controller: ExternalController<_, 20> = ExternalController::new(connector);

    let mut vibrator = Output::new(peripherals.GPIO7, Level::Low, OutputConfig::default());

    critical_section::with(|cs| VIBRATOR.borrow_ref_mut(cs).replace(vibrator));

    // Configure GPIO1 as an analog input
    let analog_pin = peripherals.GPIO5;
    let mut adc1_config = AdcConfig::new();
    let mut pin = adc1_config.enable_pin(analog_pin, Attenuation::_11dB);
    let mut adc1 = Adc::new(peripherals.ADC2, adc1_config);

    // Spawn tasks
    spawner.spawn(working()).unwrap();
    spawner.spawn(periodic_vibration()).unwrap();
    let pin_value: u16 = nb::block!(adc1.read_oneshot(&mut pin)).unwrap();
    info!("[main] ADC read value: {}", pin_value);

    run(controller, &mut rng).await;
}

#[embassy_executor::task]
async fn working() {
    loop {
        info!("{:?}", embassy_time::Instant::now().as_secs());
        embassy_time::Timer::after_secs(1).await;
    }
}

#[embassy_executor::task]
async fn periodic_vibration() {
    loop {
        let duration = critical_section::with(|cs| {
            *VIBRATION_DURATION.borrow_ref(cs).as_ref().unwrap_or(&5000) // Default to 5 seconds
        });

        critical_section::with(|cs| {
            VIBRATOR.borrow_ref_mut(cs).as_mut().unwrap().set_high();
        });

        Timer::after(Duration::from_millis(500)).await;

        critical_section::with(|cs| {
            VIBRATOR.borrow_ref_mut(cs).as_mut().unwrap().set_low();
        });

        Timer::after(Duration::from_millis(duration.into())).await;
    }
}

async fn run<C, RNG>(controller: C, random_generator: &mut RNG)
where
    C: Controller,
    RNG: RngCore + CryptoRng,
{
    let address = Address::random([0xff, 0x8f, 0x1a, 0x05, 0xe4, 0xff]);
    info!("[run] Our BLE address = {:?}", address.addr);

    let mut resources: HostResources<DefaultPacketPool, CONNECTIONS_MAX, L2CAP_CHANNELS_MAX> =
        HostResources::new();
    let stack = trouble_host::new(controller, &mut resources)
        .set_random_address(address)
        .set_random_generator_seed(random_generator);
    let Host {
        mut peripheral,
        runner,
        ..
    } = stack.build();

    info!("[run] Starting BLE advertising and GATT server setup...");
    let server = Server::new_with_config(GapConfig::Peripheral(PeripheralConfig {
        name: "CANOPY",
        appearance: &appearance::MEDIA_PLAYER,
    }))
    .unwrap();

    let _ = join(ble_task(runner), async {
        loop {
            match advertise(&mut peripheral, &server).await {
                Ok(conn) => {
                    info!("[run] Connected, spawning GATT + button tasks");
                    let _ = gatt_events_task(&server, &conn).await;
                }
                Err(e) => warn!("[adv] Advertising error: {:?}", defmt::Debug2Format(&e)),
            }
        }
    })
    .await;
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
    server: &'b Server<'_>,
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

async fn gatt_events_task<P: PacketPool>(
    server: &Server<'_>,
    conn: &GattConnection<'_, '_, P>,
) -> Result<(), Error> {
    loop {
        match conn.next().await {
            GattConnectionEvent::Bonded { bond_info } => {
                info!("bonding info: {:?}", defmt::Debug2Format(&bond_info));
            }

            GattConnectionEvent::PhyUpdated { tx_phy, rx_phy } => {
                info!("[gatt] Phy updated. Tx phy: {}, Rx phy: {}", tx_phy, rx_phy);
            }

            GattConnectionEvent::Disconnected { reason } => {
                info!("[gatt] Disconnected: {:?}", reason);
                match reason {
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
                        info!(
                            "[gatt] Disconnection due to Connection Rejected (Limited Resources)"
                        );
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
                        info!(
                            "[gatt] Disconnection due to Remote Device Terminated (Low Resources)"
                        );
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
                        info!(
                            "[gatt] Disconnection due to Connection Rejected (No Suitable Channel)"
                        );
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
                    }

                    Status::UNKNOWN_CONN_IDENTIFIER => {
                        info!("[gatt] Disconnection due to Unknown Connection Identifier");
                    }

                    _ => {
                        info!(
                            "[gatt] Disconnection due to Unknown Error {:?}",
                            defmt::Debug2Format(&reason)
                        );
                    }
                }
                embassy_time::Timer::after(Duration::from_millis(1000)).await;
                break Ok(());
            }
            GattConnectionEvent::Gatt { event } => match event {
                Ok(evt) => {
                    info!("[gatt] Received event");

                    match &evt {
                        GattEvent::Read(_) => {}
                        GattEvent::Write(write)
                            if write.handle() == server.hid.vibration_duration.handle() =>
                        {
                            let duration_seconds: u32 =
                                write.data().iter().map(|&byte| byte as u32).sum();
                            critical_section::with(|cs| {
                                *VIBRATION_DURATION.borrow_ref_mut(cs) =
                                    Some(duration_seconds * 1000); // Convert to milliseconds
                            });
                            info!(
                                "[gatt] Updated vibration duration to {} ms",
                                duration_seconds * 1000
                            );
                        }
                        GattEvent::Write(_) => {
                            info!("[gatt] Received write request");

                            critical_section::with(|cs| {
                                VIBRATOR.borrow_ref_mut(cs).as_mut().unwrap().set_high();
                            });

                            Timer::after(Duration::from_millis(1000)).await;
                            critical_section::with(|cs| {
                                VIBRATOR.borrow_ref_mut(cs).as_mut().unwrap().set_low();
                            });
                        }
                    };

                    let result = evt.accept();
                    match result {
                        Ok(reply) => {
                            info!("[gatt] Sending read's GATT response");

                            reply.send().await;
                        }
                        Err(e) => warn!(
                            "[gatt] Error sending read's response: {:?}",
                            defmt::Debug2Format(&e)
                        ),
                    }
                }
                Err(e) => warn!("[gatt] GATT event error: {:?}", defmt::Debug2Format(&e)),
            },

            GattConnectionEvent::ConnectionParamsUpdated {
                conn_interval,
                peripheral_latency,
                supervision_timeout,
            } => {
                info!("[gatt] Connection parameters updated. Conn interval(ms): {}, Peripheral latency: {}, Supervision timeout(ms): {}", conn_interval.as_millis(), peripheral_latency, supervision_timeout.as_millis());
            }
        }
    }
}
