use core::cell::RefCell;
use core::mem;

use alloc::boxed::Box;
use bt_hci::param::LeAdvReport;
use defmt::{debug, info, warn, Debug2Format};
use embassy_futures::join::join;
use embassy_time::Duration;
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
    ScanStatus,
};
use crate::system::hal::radio::AsyncRadio;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::mutex::Mutex;

const MAX_TRACKED_DEVICES: usize = 64;

#[embassy_executor::task]
pub(crate) async fn bluetooth_service(
    radio: &'static Mutex<CriticalSectionRawMutex, Box<dyn AsyncRadio>>,
) {
    info!("Bluetooth service starting");

    let mut _radio_guard = radio.lock().await;
    let stack = _radio_guard.get_stack().await;
    let Host { central, mut runner, .. } = stack.build();

    let runner_events = event_sender();
    let mut control_events = event_sender();
    let mut command_rx: CommandReceiver = command_receiver();

    let handler = ScanEventHandler::new(runner_events);

    let runner_task = runner.run_with_handler(&handler);
    let control_task = async move {
        let mut scanner = Scanner::new(central);
        let mut session: Option<ScanSession<'static, false>> = None;

        let _ = control_events
            .send(BluetoothEvent::ScanStatus(ScanStatus::Idle))
            .await;

        loop {
            let command = command_rx.receive().await;
            match command {
                BluetoothCommand::StartScan => {
                    if session.is_some() {
                        debug!("Scan request ignored; already running");
                        let _ = control_events
                            .send(BluetoothEvent::ScanStatus(ScanStatus::AlreadyRunning))
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
                                // Safety: `ScanSession` holds references into the host stack, which
                                // remains valid for the lifetime of this task because `_radio_guard`
                                // keeps the underlying radio resources pinned. We drop any active
                                // session before the guard, so widening the lifetime is sound.
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
            }
        }
    };

    join(runner_task, control_task).await;
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
