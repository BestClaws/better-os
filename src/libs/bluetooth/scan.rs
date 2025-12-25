use core::cell::RefCell;

use bt_hci::param::LeAdvReport;
use defmt::debug;
use heapless::Vec;
use trouble_host::prelude::{EventHandler, LeAdvReportsIter};

use crate::libs::bluetooth::{
    parse_device_name, BluetoothEvent, DeviceName, DiscoveredDevice, EventSender,
};

const MAX_TRACKED_DEVICES: usize = 64;

struct KnownDevice {
    addr: [u8; 6],
    name: Option<DeviceName>,
}

/// Handles BLE advertising reports and emits discovery events without duplicates.
pub struct ScanEventHandler {
    events: EventSender,
    seen: RefCell<Vec<KnownDevice, MAX_TRACKED_DEVICES>>,
}

impl ScanEventHandler {
    pub fn new(events: EventSender) -> Self {
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
