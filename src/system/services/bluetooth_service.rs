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
    command_receiver, event_sender, parse_device_name, BlePacket, BluetoothCommand, BluetoothError,
    BluetoothEvent, CommandReceiver, DeviceName, DiscoveredDevice, EventSender, GattServiceStatus,
    ScanStatus, BLE_PACKET_CAPACITY,
};
use crate::libs::http_bridge::{
    deliver_error, deliver_response, discard_pending_request, has_inflight_request,
    request_receiver as http_request_receiver, set_connection_state, HttpBridgeError,
    HttpRequestBuffer, HttpRequestReceiver, HttpResponseBuffer,
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

const HTTP_DEVICE_NAME: &str = "Better HTTP";
const HTTP_SERVICE_NAME: &str = "BLE HTTP Bridge";
const HTTP_REQUEST_CHAR_NAME: &str = "HTTP Request Channel";
const HTTP_RESPONSE_CHAR_NAME: &str = "HTTP Response Channel";
const RESPONSE_BUFFER_CAPACITY: usize = 512;
const HTTP_NOTIFY_CHUNK: usize = 20;

/// BLE HTTP bridge service definition (vendor specific UUIDs).
#[gatt_server]
struct HttpBridgeServer {
    http: HttpBridgeService,
}

/// GATT service implementing the `BLE HTTP Bridge`.
#[gatt_service(uuid = "408813df-3469-4f19-9d01-7c87f7f04001")]
struct HttpBridgeService {
    /// `HTTP Request Channel` characteristic.
    #[characteristic(
        uuid = "408813df-3469-4f19-9d01-7c87f7f04002",
        read,
        notify,
        value = BlePacket::new(),
    )]
    request: BlePacket,
    /// `HTTP Response Channel` characteristic.
    #[characteristic(
        uuid = "408813df-3469-4f19-9d01-7c87f7f04003",
        read,
        write,
        notify,
        value = BlePacket::new(),
    )]
    response: BlePacket,
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

    let gatt_task = http_gatt_service(peripheral);

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
            seen.push(KnownDevice {
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

async fn http_gatt_service<'stack, C>(mut peripheral: Peripheral<'stack, C, DefaultPacketPool>)
where
    C: Controller,
{
    let status_events = event_sender();
    let _ = status_events.try_send(BluetoothEvent::GattServiceStatus(GattServiceStatus::Idle));

    let mut server = HttpBridgeServer::new_with_config(GapConfig::Peripheral(PeripheralConfig {
        name: HTTP_DEVICE_NAME,
        appearance: &appearance::power_device::GENERIC_POWER_DEVICE,
    }))
    .unwrap();
    info!("[gatt] {} ready", HTTP_SERVICE_NAME);

    let mut request_rx = http_request_receiver();

    loop {
        gatt_acquire_exclusive().await;

        let _ = status_events.try_send(BluetoothEvent::GattServiceStatus(
            GattServiceStatus::Advertising,
        ));
        info!(
            "[gatt] advertising {} as {}",
            HTTP_SERVICE_NAME, HTTP_DEVICE_NAME
        );

        match advertise_http(&mut peripheral, &server).await {
            Ok(conn) => {
                info!("[gatt] {} connected", HTTP_SERVICE_NAME);
                let _ = status_events.try_send(BluetoothEvent::GattServiceStatus(
                    GattServiceStatus::Connected,
                ));
                set_connection_state(true);

                let session_status = event_sender();
                let tx_events = event_sender();
                let rx_events = event_sender();

                if let Err(err) = http_client_session(
                    &server.http.request,
                    &server.http.response,
                    &conn,
                    session_status,
                    tx_events,
                    rx_events,
                    &mut request_rx,
                )
                .await
                {
                    warn!("[gatt] HTTP session error: {:?}", Debug2Format(&err));
                    let _ = status_events
                        .try_send(BluetoothEvent::GattServiceStatus(GattServiceStatus::Error));
                }

                set_connection_state(false);

                if has_inflight_request() {
                    deliver_error(HttpBridgeError::Disconnected);
                }

                while discard_pending_request(&mut request_rx) {
                    deliver_error(HttpBridgeError::Disconnected);
                }

                Timer::after_millis(500).await;
            }
            Err(err) => {
                warn!("[gatt] advertising failed: {:?}", Debug2Format(&err));
                let _ = status_events
                    .try_send(BluetoothEvent::GattServiceStatus(GattServiceStatus::Error));
                Timer::after_millis(1_000).await;
            }
        }

        set_connection_state(false);
        let _ = status_events.try_send(BluetoothEvent::GattServiceStatus(GattServiceStatus::Idle));
        gatt_release_exclusive().await;
    }
}

async fn http_client_session(
    request_char: &Characteristic<BlePacket>,
    response_char: &Characteristic<BlePacket>,
    conn: &GattConnection<'_, '_, DefaultPacketPool>,
    status_events: EventSender,
    tx_events: EventSender,
    rx_events: EventSender,
    request_rx: &mut HttpRequestReceiver,
) -> Result<(), Error> {
    let mut response_buffer: Vec<u8, RESPONSE_BUFFER_CAPACITY> = Vec::new();
    let mut reported_complete = false;
    let mut notifications_enabled = request_char.cccd_handle.map(|_| false).unwrap_or(true);
    let mut request_in_flight = false;
    let mut pending_request: Option<HttpRequestBuffer> = None;

    loop {
        if notifications_enabled && !request_in_flight && pending_request.is_none() {
            match select(request_rx.receive(), conn.next()).await {
                Either::First(request) => {
                    pending_request = Some(request);
                    response_buffer.clear();
                    reported_complete = false;
                }
                Either::Second(event) => {
                    if handle_connection_event(
                        event,
                        request_char,
                        response_char,
                        &status_events,
                        &rx_events,
                        &mut response_buffer,
                        &mut reported_complete,
                        &mut request_in_flight,
                        &mut notifications_enabled,
                    )
                    .await?
                    {
                        break;
                    }
                    continue;
                }
            }
        } else {
            let event = conn.next().await;
            if handle_connection_event(
                event,
                request_char,
                response_char,
                &status_events,
                &rx_events,
                &mut response_buffer,
                &mut reported_complete,
                &mut request_in_flight,
                &mut notifications_enabled,
            )
            .await?
            {
                break;
            }
            continue;
        }

        if let Some(request) = pending_request.take() {
            let _ = status_events.try_send(BluetoothEvent::GattServiceStatus(
                GattServiceStatus::SendingRequest,
            ));
            if let Err(err) =
                send_http_request(request_char, conn, &tx_events, request.as_slice()).await
            {
                warn!(
                    "[gatt] failed to send HTTP request: {:?}",
                    Debug2Format(&err)
                );
                deliver_error(HttpBridgeError::Internal);
                return Err(err);
            }
            let _ = status_events.try_send(BluetoothEvent::GattServiceStatus(
                GattServiceStatus::AwaitingResponse,
            ));
            request_in_flight = true;
        }
    }

    if pending_request.is_some() {
        deliver_error(HttpBridgeError::Disconnected);
    }

    Ok(())
}

#[allow(clippy::too_many_arguments)]
async fn handle_connection_event(
    event: GattConnectionEvent<'_, '_, DefaultPacketPool>,
    request_char: &Characteristic<BlePacket>,
    response_char: &Characteristic<BlePacket>,
    status_events: &EventSender,
    rx_events: &EventSender,
    response_buffer: &mut Vec<u8, RESPONSE_BUFFER_CAPACITY>,
    reported_complete: &mut bool,
    request_in_flight: &mut bool,
    notifications_enabled: &mut bool,
) -> Result<bool, Error> {
    match event {
        GattConnectionEvent::Disconnected { reason } => {
            info!("[gatt] disconnected: {:?}", reason);
            if *request_in_flight {
                deliver_error(HttpBridgeError::Disconnected);
            }
            if !response_buffer.is_empty() && !*reported_complete {
                let _ = status_events.try_send(BluetoothEvent::GattServiceStatus(
                    GattServiceStatus::ResponseComplete,
                ));
            }
            *request_in_flight = false;
            response_buffer.clear();
            *reported_complete = false;
            return Ok(true);
        }
        GattConnectionEvent::Gatt { event } => {
            if let GattEvent::Write(write) = &event {
                if Some(write.handle()) == request_char.cccd_handle {
                    let enabled = write
                        .data()
                        .first()
                        .map(|flags| flags & 0x01 != 0)
                        .unwrap_or(false);
                    *notifications_enabled = enabled;

                    debug!(
                        "[gatt] {} notifications {}",
                        HTTP_REQUEST_CHAR_NAME,
                        if *notifications_enabled {
                            "enabled"
                        } else {
                            "disabled"
                        }
                    );

                    if !*notifications_enabled {
                        *request_in_flight = false;
                        response_buffer.clear();
                        *reported_complete = false;
                        let _ = status_events.try_send(BluetoothEvent::GattServiceStatus(
                            GattServiceStatus::Connected,
                        ));
                    }
                } else if write.handle() == response_char.handle {
                    debug!(
                        "[gatt] received {} chunk ({} bytes)",
                        HTTP_RESPONSE_CHAR_NAME,
                        write.data().len()
                    );

                    if response_buffer.extend_from_slice(write.data()).is_err() {
                        warn!("[gatt] response truncated (buffer full)");
                    }

                    match BlePacket::from_slice(write.data()) {
                        Ok(packet) => {
                            let _ = rx_events.try_send(BluetoothEvent::GattValueReceived(packet));
                        }
                        Err(_) => {
                            let mut packet = BlePacket::new();
                            let copy_len = write.data().len().min(BLE_PACKET_CAPACITY);
                            packet.extend_from_slice(&write.data()[..copy_len]).ok();
                            let _ = rx_events.try_send(BluetoothEvent::GattValueReceived(packet));
                        }
                    }

                    if !*reported_complete {
                        let mut delivered: HttpResponseBuffer = HttpResponseBuffer::new();
                        let copy_len = response_buffer.len().min(delivered.capacity());
                        delivered
                            .extend_from_slice(&response_buffer[..copy_len])
                            .ok();
                        if response_buffer.len() > delivered.capacity() {
                            warn!(
                                "[gatt] delivered response truncated ({} / {})",
                                copy_len,
                                response_buffer.len()
                            );
                        }
                        deliver_response(delivered);
                        let _ = status_events.try_send(BluetoothEvent::GattServiceStatus(
                            GattServiceStatus::ResponseComplete,
                        ));
                        *reported_complete = true;
                        *request_in_flight = false;
                    }
                }
            }

            if let Ok(reply) = event.accept() {
                reply.send().await;
            }
        }
        _ => {}
    }

    Ok(false)
}

async fn send_http_request(
    request_char: &Characteristic<BlePacket>,
    conn: &GattConnection<'_, '_, DefaultPacketPool>,
    events: &EventSender,
    payload: &[u8],
) -> Result<(), Error> {
    let mut chunk_index = 0u32;
    for chunk in payload.chunks(HTTP_NOTIFY_CHUNK) {
        let mut packet = BlePacket::new();
        if packet.extend_from_slice(chunk).is_err() {
            warn!("[gatt] request chunk truncated");
            packet.extend_from_slice(&chunk[..BLE_PACKET_CAPACITY]).ok();
        }

        debug!(
            "[gatt] sending {} chunk #{} ({} bytes)",
            HTTP_REQUEST_CHAR_NAME,
            chunk_index,
            packet.len()
        );
        let event_packet = packet.clone();
        request_char.notify(conn, &packet).await?;
        let _ = events.try_send(BluetoothEvent::GattValueSent(event_packet));
        chunk_index = chunk_index.saturating_add(1);
    }

    Ok(())
}

async fn advertise_http<'values, 'server, C>(
    peripheral: &mut Peripheral<'values, C, DefaultPacketPool>,
    server: &'server HttpBridgeServer<'values>,
) -> Result<GattConnection<'values, 'server, DefaultPacketPool>, BleHostError<C::Error>>
where
    C: Controller,
{
    let mut adv_data = [0u8; 31];
    let len = AdStructure::encode_slice(
        &[
            AdStructure::Flags(LE_GENERAL_DISCOVERABLE | BR_EDR_NOT_SUPPORTED),
            AdStructure::CompleteLocalName(HTTP_DEVICE_NAME.as_bytes()),
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
    info!("[gatt] {} connection established", HTTP_SERVICE_NAME);
    Ok(conn)
}

async fn gatt_acquire_exclusive() {
    GATT_CONTROL.signal(GattControl::Acquire);
    GATT_ACK.wait().await;
}

async fn gatt_release_exclusive() {
    GATT_CONTROL.signal(GattControl::Release);
    GATT_ACK.wait().await;
}
