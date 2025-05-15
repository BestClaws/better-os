//! BLE HID Keyboard example on ESP32 (three buttons → F7, F8, F9)
#![no_std]
#![no_main]

use bt_hci::controller::ExternalController;
use defmt::{info, warn};
use embassy_executor::Spawner;
use embassy_futures::{join::join, select::select};
use embassy_futures::select::Either;
use embassy_time::{Duration, Timer};
use esp_hal::clock::CpuClock;
use esp_hal::gpio::{Input, InputConfig, Io, Level, Output, OutputConfig, Pull};
use esp_hal::timer::systimer::SystemTimer;
use esp_hal::timer::timg::TimerGroup;
use esp_wifi::ble::controller::BleConnector;
use trouble_host::{prelude::*, Address, Host, HostResources};
use panic_rtt_target as _;

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

#[gatt_service(uuid = service::HUMAN_INTERFACE_DEVICE)]
struct HidService {
    #[characteristic(uuid = BluetoothUuid16::new(0x2A4A), read, value = [0x01, 0x11, 0x00, 0x03])]
    information: [u8; 4],

    #[characteristic(uuid = BluetoothUuid16::new(0x2A4B), read, value = [
        0x05, 0x01, 0x09, 0x06, 0xA1, 0x01, 0x75, 0x01, 0x95, 0x08,
        0x05, 0x07, 0x19, 0xE0, 0x29, 0xE7, 0x15, 0x00, 0x25, 0x01,
        0x81, 0x02, 0x95, 0x01, 0x75, 0x08, 0x81, 0x01, 0x95, 0x06,
        0x75, 0x08, 0x15, 0x00, 0x25, 0x65, 0x05, 0x07, 0x19, 0x00,
        0x29, 0x65, 0x81, 0x00, 0xC0
    ])]
    report_map: [u8; 45],

    #[characteristic(uuid = BluetoothUuid16::new(0x2A4D), read, notify, value = [0; 8])]
    input_report: [u8; 8],

    #[characteristic(uuid = BluetoothUuid16::new(0x2A4E), read, write, value = 0x00)]
    protocol_mode: u8,

    #[characteristic(uuid = BluetoothUuid16::new(0x2A22), read, value = [0; 8])]
    boot_keyboard_input: [u8; 8],
}

extern crate alloc;

#[esp_hal_embassy::main]
async fn main(_spawner: Spawner) {
    rtt_target::rtt_init_defmt!();

    let config = esp_hal::Config::default().with_cpu_clock(CpuClock::max());
    let peripherals = esp_hal::init(config);
    esp_alloc::heap_allocator!(size: 72 * 1024);

    let timer0 = SystemTimer::new(peripherals.SYSTIMER);
    esp_hal_embassy::init(timer0.alarm0);
    info!("[main] Embassy initialized");

    let timer1 = TimerGroup::new(peripherals.TIMG0);
    let init = esp_wifi::init(
        timer1.timer0,
        esp_hal::rng::Rng::new(peripherals.RNG),
        peripherals.RADIO_CLK,
    )
        .unwrap();

    let connector = BleConnector::new(&init, peripherals.BT);
    let controller: ExternalController<_, 20> = ExternalController::new(connector);

    let io = Io::new(peripherals.IO_MUX);
    let mut led = Output::new(peripherals.GPIO0, Level::Low, OutputConfig::default());
    let mut b1 = Input::new(peripherals.GPIO2, InputConfig::default().with_pull(Pull::Up));
    let mut b2 = Input::new(peripherals.GPIO3, InputConfig::default().with_pull(Pull::Up));
    let mut b3 = Input::new(peripherals.GPIO4, InputConfig::default().with_pull(Pull::Up));

    run(controller, &mut b1, &mut b2, &mut b3, &mut led).await;
}

async fn run<C>(
    controller: C,
    b1: &mut Input<'_>,
    b2: &mut Input<'_>,
    b3: &mut Input<'_>,
    led: &mut Output<'_>,
)
where
    C: Controller,
{
    let address = Address::random([0xff, 0x8f, 0x1a, 0x05, 0xe4, 0xff]);
    info!("[run] Our BLE address = {:?}", address.addr);

    let mut resources: HostResources<DefaultPacketPool, CONNECTIONS_MAX, L2CAP_CHANNELS_MAX> = HostResources::new();
    let stack = trouble_host::new(controller, &mut resources).set_random_address(address);
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
    let len = AdStructure::encode_slice(
        &[
            AdStructure::Flags(LE_GENERAL_DISCOVERABLE | BR_EDR_NOT_SUPPORTED),
            AdStructure::ServiceUuids16(&[[0x12, 0x18]]),
            AdStructure::CompleteLocalName(b"ESP32-HID"),
        ],
        &mut adv_data,
    )?;

    let adv = peripheral
        .advertise(&AdvertisementParameters::default(),
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
        match conn.next().await {
            GattConnectionEvent::Disconnected { reason } => {
                info!("[gatt] Disconnected: {:?}", reason);
                break;
            }
            GattConnectionEvent::Gatt { event } => match event {
                Ok(evt) => {
                    info!("[gatt] Received event");
                    match evt.accept() {
                        Ok(mut reply) => {
                            info!("[gatt] Sending GATT response");
                            reply.send().await;
                        }
                        Err(e) => warn!("[gatt] Error sending response: {:?}", defmt::Debug2Format(&e)),
                    }
                }
                Err(e) => warn!("[gatt] GATT event error: {:?}", defmt::Debug2Format(&e)),
            },
            other => info!("[gatt] Unexpected event:"),
        }
    }
    Ok(())
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
            async { b1.wait_for_low().await; KEY_A },
            select(
                async { b2.wait_for_low().await; KEY_B },
                async { b3.wait_for_low().await; KEY_C },
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
