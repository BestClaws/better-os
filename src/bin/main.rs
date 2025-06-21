//! BLE HID Keyboard example on ESP32 (three buttons → F7, F8, F9)
#![no_std]
#![no_main]

extern crate alloc;
use core::cell::RefCell;
use esp_hal::peripherals::{ADC1, GPIO2};
use esp_hal::Blocking;
use nb;

use bt_hci::controller::ExternalController;
use critical_section::Mutex;
use defmt::{info, warn};
use embassy_executor::Spawner;
use embassy_futures::join::join;
use embassy_futures::select::select;
use embassy_sync::blocking_mutex::raw::{CriticalSectionRawMutex};
use embassy_time::{Duration, Timer};
use esp_hal::analog::adc::{Adc, AdcConfig, AdcPin, Attenuation};
use esp_hal::clock::CpuClock;
use esp_hal::gpio::{Level, Output, OutputConfig};
use esp_hal::timer::systimer::SystemTimer;
use esp_hal::timer::timg::TimerGroup;
use esp_wifi::ble::controller::BleConnector;
use panic_rtt_target as _;
use trouble_host::{prelude::*, Address, Host, HostResources};
use embassy_sync::signal::Signal;


/// Max number of connections
const CONNECTIONS_MAX: usize = 1;
/// Max number of L2CAP channels (Signal and ATT)
const L2CAP_CHANNELS_MAX: usize = 2;

static VIBRATOR: Mutex<RefCell<Option<Output>>> = Mutex::new(RefCell::new(None));
static PERIOD_VIBRATION_LENGTH: u64 = 1000;
static VIBRATION_PERIOD_UPDATE_SIG: Signal<CriticalSectionRawMutex, u32> = Signal::new();




// GATT Server definition: HID Service
#[gatt_server]
struct Server {
    hid: CanopyService,
}

// #[gatt_service(uuid = "12345678-1234-5678-1234-56789abcdef0")]
#[gatt_service(uuid = BluetoothUuid16::new(0xAAAA))]
struct CanopyService {

    #[characteristic(uuid = BluetoothUuid16::new(0xAAAB), read, write)]
    #[descriptor(uuid = BluetoothUuid16::new(0x2901), read, value = "Amount")]
    amount: u8,

    #[characteristic(uuid = BluetoothUuid16::new(0xAAAC), write)]
    #[descriptor(uuid = BluetoothUuid16::new(0x2901), read, value = "Vibration Duration")]
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

    let rng = esp_hal::rng::Rng::new(peripherals.RNG);

    let timer1 = TimerGroup::new(peripherals.TIMG0);
    let init = esp_wifi::init(
        timer1.timer0,
        rng.clone(),
        peripherals.RADIO_CLK,
    ).unwrap();

    let connector = BleConnector::new(&init, peripherals.BT);
    let controller: ExternalController<_, 20> = ExternalController::new(connector);

    let vibrator = Output::new(peripherals.GPIO7, Level::Low, OutputConfig::default());

    critical_section::with(|cs| {
        VIBRATOR.borrow_ref_mut(cs).replace(vibrator)
    });

    spawner.spawn(working()).unwrap();
    spawner.spawn(periodic_vibration()).unwrap(); // Spawn the new vibration task

    let analog_pin = peripherals.GPIO2;
    let mut adc1_config = AdcConfig::new();
    let mut batt_pin = adc1_config.enable_pin(analog_pin, Attenuation::_11dB);
    let mut adc1 = Adc::new(peripherals.ADC1, adc1_config);
    let pin_value: u16 = nb::block!(adc1.read_oneshot(&mut batt_pin)).unwrap();
    info!("[main] ADC read value: {}", pin_value);

    run(controller, adc1, batt_pin).await;

}

#[embassy_executor::task]
async fn working() {
    loop {
        info!("{:?}", embassy_time::Instant::now().as_secs());
        Timer::after_secs(1).await;
    }
}

#[embassy_executor::task]
async fn periodic_vibration() {
    let mut duration = Duration::from_millis(5000);
    loop {

        info!("[periodic_vibration] Waiting for signal");

        critical_section::with(|cs| {
            VIBRATOR
                .borrow_ref_mut(cs)
                .as_mut()
                .unwrap()
                .set_high();
        });

        Timer::after(Duration::from_millis(PERIOD_VIBRATION_LENGTH)).await;

        critical_section::with(|cs| {
            VIBRATOR
                .borrow_ref_mut(cs)
                .as_mut()
                .unwrap()
                .set_low();
        });

        select(
            Timer::after(Duration::from_millis(duration.as_millis())),
            async {
                duration = Duration::from_millis(VIBRATION_PERIOD_UPDATE_SIG.wait().await.into());
            }
        ).await;

    }
}

async fn run<C>(
    controller: C,
    mut adc: Adc<'static, ADC1<'static>, Blocking>,
    mut batt_pin: AdcPin<GPIO2<'static>, ADC1<'static>>
) where
    C: Controller

{
    let address = Address::random([0xff, 0x8f, 0x1a, 0x05, 0xe4, 0xf0]);
    info!("[run] Our BLE address = {:?}", address.addr);

    let mut resources: HostResources<DefaultPacketPool, CONNECTIONS_MAX, L2CAP_CHANNELS_MAX> = HostResources::new();
    let stack = trouble_host::new(controller, &mut resources).set_random_address(address);
    let Host { mut peripheral, runner, .. } = stack.build();

    info!("[run] Starting BLE advertising and GATT server setup...");
    let server = Server::new_with_config(GapConfig::Peripheral(PeripheralConfig {
        name: "CANOPY",
        appearance: &appearance::MEDIA_PLAYER,

    })).unwrap();

    let _ = join(
        ble_task(runner),
        async {
            loop {
                match advertise(&mut peripheral, &server).await {
                    Ok(conn) => {
                        info!("[run] Connected, spawning GATT + button tasks");
                        let _ = gatt_events_task(&server, &conn, &mut adc, &mut batt_pin).await;
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
    server: &'b Server<'_>
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
    adc: &mut Adc<'static, ADC1<'static>, Blocking>,
    batt_pin: &mut AdcPin<GPIO2<'static>, ADC1<'static>>
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
                    info!("[gatt] Received event");

                    match &event {
                        GattEvent::Read(_) => {

                            let value = match nb::block!(adc.read_oneshot(batt_pin)) {
                                Ok(v) => {
                                    let val = (v >> 4) as u8; // Scale 12-bit ADC (0–4095) to 8-bit (0–255)
                                    info!("[gatt] Read from ADC = {}, scaled = {}", v, val);
                                    val
                                }
                                Err(_) => {
                                    warn!("[gatt] Failed to read ADC");
                                    0
                                }
                            };

                            let _ = server.hid.amount.set(server, &value);

                        }
                        GattEvent::Write(write) if write.handle() == server.hid.vibration_duration.handle() => {
                            let duration_seconds: u32 = write.data().iter().map(|&byte| byte as u32).sum();

                            VIBRATION_PERIOD_UPDATE_SIG.signal(duration_seconds * 1000);

                            //
                            // critical_section::with(|cs| {
                            //     *VIBRATION_DURATION.borrow_ref_mut(cs) = Some(duration_seconds * 1000); // Convert to milliseconds
                            // });

                            info!("[gatt] Updated vibration duration to {} ms", duration_seconds * 1000);
                        }
                        GattEvent::Write(_) => {

                            info!("[gatt] Received write request");


                            critical_section::with(|cs| {
                                VIBRATOR
                                    .borrow_ref_mut(cs)
                                    .as_mut()
                                    .unwrap()
                                    .set_high();
                            });

                            Timer::after(Duration::from_millis(1000)).await;
                            critical_section::with(|cs| {
                                VIBRATOR
                                    .borrow_ref_mut(cs)
                                    .as_mut()
                                    .unwrap()
                                    .set_low();
                            });


                        },
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
