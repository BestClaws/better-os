use core::mem;

use alloc::boxed::Box;
use defmt::{debug, info, warn, Debug2Format};
use embassy_futures::join::join;
use embassy_futures::select::{select, Either};
use embassy_time::{Duration, Timer};
use trouble_host::prelude::*;

use crate::libs::bluetooth::{
    command_receiver, event_sender, BluetoothCommand, BluetoothError, BluetoothEvent,
    CommandReceiver, GattServiceStatus, ScanEventHandler, ScanStatus,
};
use crate::libs::http_bridge::gatt::{
    advertise_http, http_client_session, HttpBridgeServer, HTTP_DEVICE_NAME, HTTP_SERVICE_NAME,
};
use crate::libs::http_bridge::{
    deliver_error, discard_pending_request, has_inflight_request,
    request_receiver as http_request_receiver, set_connection_state, HttpBridgeError,
};
use crate::system::hal::radio::AsyncRadio;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::mutex::Mutex;
use embassy_sync::signal::Signal;

enum GattControl {
    Acquire,
    Release,
}

static GATT_CONTROL: Signal<CriticalSectionRawMutex, GattControl> = Signal::new();
static GATT_ACK: Signal<CriticalSectionRawMutex, ()> = Signal::new();

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

async fn gatt_acquire_exclusive() {
    GATT_CONTROL.signal(GattControl::Acquire);
    GATT_ACK.wait().await;
}

async fn gatt_release_exclusive() {
    GATT_CONTROL.signal(GattControl::Release);
    GATT_ACK.wait().await;
}
