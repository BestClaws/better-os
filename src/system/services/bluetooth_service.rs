use core::cell::RefCell;
use core::mem;

use alloc::boxed::Box;
use bt_hci::param::LeAdvReport;
use defmt::{debug, info, warn, Debug2Format};
use embassy_futures::join::join;
use embassy_futures::select::{select, Either};
use embassy_time::{Duration, Timer};
use heapless::Vec;
use trouble_host::prelude::*;

use crate::libs::bluetooth::{
    command_receiver,
    event_sender,
    parse_device_name,
    BluetoothCommand,
    BluetoothError,
    BluetoothEvent,
    CommandReceiver,
    DeviceName,
    DiscoveredDevice,
    EventSender,
    GattServiceStatus,
    ScanStatus,
};
use crate::system::hal::radio::AsyncRadio;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::mutex::Mutex;
use embassy_sync::signal::Signal;

const MAX_TRACKED_DEVICES: usize = 64;

enum GattControl {
    Acquire,
    Release,
}

static GATT_CONTROL: Signal<CriticalSectionRawMutex, GattControl> = Signal::new();
static GATT_ACK: Signal<CriticalSectionRawMutex, ()> = Signal::new();

/// BLE random value service definition (vendor specific UUIDs).
#[gatt_server]
struct RandomServiceServer {
    random: RandomValueService,
}

#[gatt_service(uuid = "408813df-3469-4f19-9d01-7c87f7f04001")]
struct RandomValueService {
    #[characteristic(uuid = "408813df-3469-4f19-9d01-7c87f7f04002", read, notify, value = 0)]
    outbound: u8,
    #[characteristic(uuid = "408813df-3469-4f19-9d01-7c87f7f04003", read, write, notify, value = 0)]
    inbound: u8,
}

#[embassy_executor::task]
pub(crate) async fn bluetooth_service(
    radio: &'static Mutex<CriticalSectionRawMutex, Box<dyn AsyncRadio>>,
) {
    info!("Bluetooth service starting");

    let mut _radio_guard = radio.lock().await;
    let stack = _radio_guard.get_stack().await;
    let Host {
        central,
        peripheral,
        mut runner,
        ..
    } = stack.build();

    let runner_events = event_sender();
    let mut control_events = event_sender();
    let mut command_rx: CommandReceiver = command_receiver();

    let handler = ScanEventHandler::new(runner_events);

    let runner_task = runner.run_with_handler(&handler);
    let control_task = async move {
        let mut scanner = Scanner::new(central);
        let mut session: Option<ScanSession<'static, false>> = None;
        let mut scan_requested = false;
        let mut gatt_exclusive = false;

        let _ = control_events
            .send(BluetoothEvent::ScanStatus(ScanStatus::Idle))
            .await;

        loop {
            match select(command_rx.receive(), GATT_CONTROL.wait()).await {
                Either::First(command) => match command {
                    BluetoothCommand::StartScan => {
                        scan_requested = true;

                        if session.is_some() {
                            debug!("Scan request ignored; already running");
                            let _ = control_events
                                .send(BluetoothEvent::ScanStatus(ScanStatus::AlreadyRunning))
                                .await;
                            continue;
                        }

                        if gatt_exclusive {
                            let _ = control_events
                                .send(BluetoothEvent::ScanStatus(ScanStatus::BlockedByPeripheral))
                                .await;
                            continue;
                        }

                        let _ = control_events
                            .send(BluetoothEvent::ScanStatus(ScanStatus::Starting))
                            .await;

                        let mut config = ScanConfig::default();
                        config.active = true;
                        config.phys = PhySet::M1;
                        config.interval = Duration::from_millis(500);
                        config.window = Duration::from_millis(500);

                        match scanner.scan(&config).await {
                            Ok(scan_session) => {
                                session = Some(unsafe {
                                    mem::transmute::<
                                        ScanSession<'_, false>,
                                        ScanSession<'static, false>,
                                    >(scan_session)
                                });
                                let _ = control_events
                                    .send(BluetoothEvent::ScanStatus(ScanStatus::Running))
                                    .await;
                            }
                            Err(err) => {
                                warn!("Failed to start scan: {:?}", Debug2Format(&err));
                                let _ = control_events
                                    .send(BluetoothEvent::ScanStatus(ScanStatus::Failed(
                                        BluetoothError::OperationFailed,
                                    )))
                                    .await;
                            }
                        }
                    }
                    BluetoothCommand::StopScan => {
                        scan_requested = false;

                        if session.is_none() {
                            continue;
                        }

                        let _ = control_events
                            .send(BluetoothEvent::ScanStatus(ScanStatus::Stopping))
                            .await;
                        session.take();
                        let _ = control_events
                            .send(BluetoothEvent::ScanStatus(ScanStatus::Idle))
                            .await;
                    }
                },
                Either::Second(control) => match control {
                    GattControl::Acquire => {
                        gatt_exclusive = true;
                        if session.is_some() {
                            session.take();
                            let _ = control_events
                                .send(BluetoothEvent::ScanStatus(ScanStatus::Idle))
                                .await;
                        }
                        GATT_ACK.signal(());
                    }
                    GattControl::Release => {
                        gatt_exclusive = false;
                        GATT_ACK.signal(());

                        if scan_requested && session.is_none() {
                            let _ = control_events
                                .send(BluetoothEvent::ScanStatus(ScanStatus::Starting))
                                .await;

                            let mut config = ScanConfig::default();
                            config.active = true;
                            config.phys = PhySet::M1;
                            config.interval = Duration::from_millis(500);
                            config.window = Duration::from_millis(500);

                            match scanner.scan(&config).await {
                                Ok(scan_session) => {
                                    session = Some(unsafe {
                                        mem::transmute::<
                                            ScanSession<'_, false>,
                                            ScanSession<'static, false>,
                                        >(scan_session)
                                    });
                                    let _ = control_events
                                        .send(BluetoothEvent::ScanStatus(ScanStatus::Running))
                                        .await;
                                }
                                Err(err) => {
                                    warn!("Failed to restart scan: {:?}", Debug2Format(&err));
                                    let _ = control_events
                                        .send(BluetoothEvent::ScanStatus(ScanStatus::Failed(
                                            BluetoothError::OperationFailed,
                                        )))
                                        .await;
                                }
                            }
                        }
                    }
                },
            }
        }
    };
    let gatt_task = random_gatt_service(peripheral);

    join(runner_task, async {
        join(control_task, gatt_task).await;
    })
    .await;
}

struct KnownDevice {
    addr: [u8; 6],
    name: Option<DeviceName>,
}

struct ScanEventHandler {
    events: EventSender,
    seen: RefCell<Vec<KnownDevice, MAX_TRACKED_DEVICES>>,
}

impl ScanEventHandler {
    fn new(events: EventSender) -> Self {
        Self {
            events,
            seen: RefCell::new(Vec::new()),
        }
    }

    fn handle_report(&self, report: &LeAdvReport<'_>) {
        let mut addr = [0u8; 6];
        addr.copy_from_slice(report.addr.raw());
        let name = parse_device_name(report.data);
        let rssi = report.rssi;

        let mut seen = self.seen.borrow_mut();
        if let Some(entry) = seen.iter_mut().find(|entry| entry.addr == addr) {
            let mut emit_update = false;
            match (&entry.name, &name) {
                (Some(old), Some(new)) if old != new => {
                    entry.name = name.clone();
                    emit_update = true;
                }
                (None, Some(_)) => {
                    entry.name = name.clone();
                    emit_update = true;
                }
                _ => {}
            }

            if emit_update {
                self.emit_device(addr, name, rssi);
            }
        } else {
            if seen.is_full() {
                let _ = seen.remove(0);
            }
            seen
                .push(KnownDevice {
                    addr,
                    name: name.clone(),
                })
                .ok();
            self.emit_device(addr, name, rssi);
        }
    }

    fn emit_device(&self, addr: [u8; 6], name: Option<DeviceName>, rssi: i8) {
        let event = BluetoothEvent::DeviceDiscovered(DiscoveredDevice::new(addr, name, rssi));
        if self.events.try_send(event).is_err() {
            debug!("Bluetooth event queue full; dropping discovery event");
        }
    }
}

impl EventHandler for ScanEventHandler {
    fn on_adv_reports(&self, mut reports: LeAdvReportsIter<'_>) {
        while let Some(Ok(report)) = reports.next() {
            self.handle_report(&report);
        }
    }
}

async fn random_gatt_service<'stack, C>(
    mut peripheral: Peripheral<'stack, C, DefaultPacketPool>,
)
where
    C: Controller,
{
    let mut status_events = event_sender();
    let _ = status_events
        .try_send(BluetoothEvent::GattServiceStatus(GattServiceStatus::Idle));

    let mut server = RandomServiceServer::new_with_config(GapConfig::Peripheral(PeripheralConfig {
        name: "Better Random",
        appearance: &appearance::power_device::GENERIC_POWER_DEVICE,
    }))
    .unwrap();

    loop {
        gatt_acquire_exclusive().await;

        let _ = status_events
            .try_send(BluetoothEvent::GattServiceStatus(GattServiceStatus::Advertising));

        let advertise_result = advertise_random(&mut peripheral, &server).await;

        match advertise_result {
            Ok(conn) => {
                let _ = status_events
                    .try_send(BluetoothEvent::GattServiceStatus(GattServiceStatus::Connected));

                let outbound = server.random.outbound;
                let inbound = server.random.inbound;
                let notify_sender = event_sender();
                let inbound_sender = event_sender();

                let notify_task = notify_random_values(outbound, &conn, notify_sender);
                let inbound_task = handle_random_writes(inbound, &conn, inbound_sender);

                let _ = select(notify_task, inbound_task).await;
            }
            Err(err) => {
                warn!("[gatt] advertising failed: {:?}", Debug2Format(&err));
                let _ = status_events
                    .try_send(BluetoothEvent::GattServiceStatus(GattServiceStatus::Error));
                Timer::after_millis(1_000).await;
            }
        }

        let _ = status_events
            .try_send(BluetoothEvent::GattServiceStatus(GattServiceStatus::Idle));
        gatt_release_exclusive().await;
    }
}

async fn notify_random_values(
    characteristic: Characteristic<u8>,
    conn: &GattConnection<'_, '_, DefaultPacketPool>,
    events: EventSender,
) {
    let mut generator = Lfsr::new(0xACE1);

    loop {
        let value = generator.next();

        match characteristic.notify(conn, &value).await {
            Ok(()) => {
                let _ = events.try_send(BluetoothEvent::GattValueSent(value));
            }
            Err(err) => {
                warn!("[gatt] notify failed: {:?}", Debug2Format(&err));
                break;
            }
        }

        Timer::after_secs(2).await;
    }
}

async fn handle_random_writes(
    inbound: Characteristic<u8>,
    conn: &GattConnection<'_, '_, DefaultPacketPool>,
    events: EventSender,
) {
    loop {
        match conn.next().await {
            GattConnectionEvent::Disconnected { reason } => {
                info!("[gatt] disconnected: {:?}", reason);
                break;
            }
            GattConnectionEvent::Gatt { event } => {
                match &event {
                    GattEvent::Write(write) => {
                        if write.handle() == inbound.handle {
                            if let Some(value) = write.data().first().copied() {
                                let _ = events.try_send(BluetoothEvent::GattValueReceived(value));
                            }
                        }
                    }
                    _ => {}
                }

                if let Ok(reply) = event.accept() {
                    reply.send().await;
                }
            }
            _ => {}
        }
    }
}

async fn advertise_random<'values, 'server, C>(
    peripheral: &mut Peripheral<'values, C, DefaultPacketPool>,
    server: &'server RandomServiceServer<'values>,
) -> Result<GattConnection<'values, 'server, DefaultPacketPool>, BleHostError<C::Error>>
where
    C: Controller,
{
    let mut adv_data = [0u8; 31];
    let len = AdStructure::encode_slice(
        &[
            AdStructure::Flags(LE_GENERAL_DISCOVERABLE | BR_EDR_NOT_SUPPORTED),
            AdStructure::CompleteLocalName(b"Better Random"),
        ],
        &mut adv_data,
    )?;

    let advertiser = peripheral
        .advertise(
            &Default::default(),
            Advertisement::ConnectableScannableUndirected {
                adv_data: &adv_data[..len],
                scan_data: &[],
            },
        )
        .await?;

    let conn = advertiser.accept().await?.with_attribute_server(server)?;
    info!("[gatt] connection established");
    Ok(conn)
}

struct Lfsr {
    state: u16,
}

impl Lfsr {
    const fn new(seed: u16) -> Self {
        Self { state: if seed == 0 { 0xACE1 } else { seed } }
    }

    fn next(&mut self) -> u8 {
        if self.state == 0 {
            self.state = 0xACE1;
        }

        let bit = ((self.state >> 0) ^ (self.state >> 2) ^ (self.state >> 3) ^ (self.state >> 5)) & 1;
        self.state = (self.state >> 1) | (bit << 15);
        (self.state & 0xFF) as u8
    }
}

async fn gatt_acquire_exclusive() {
    GATT_CONTROL.signal(GattControl::Acquire);
    GATT_ACK.wait().await;
}

async fn gatt_release_exclusive() {
    GATT_CONTROL.signal(GattControl::Release);
    GATT_ACK.wait().await;
}
