use bt_hci::param::LeAdvReport;
use defmt::info;
use trouble_host::prelude::{AddrKind, EventHandler, LeAdvReportsIter};

use crate::libs::hps::types::HpsUuids;

use alloc::string::String;
use alloc::vec::Vec;
use core::cell::RefCell;

#[derive(Clone)]
pub(crate) struct ScanObservation {
    pub(crate) addr: [u8; 6],
    pub(crate) addr_kind: AddrKind,
    pub(crate) name: Option<String>,
    pub(crate) has_hps: bool,
}

pub(crate) struct HpsScanHandler {
    found_addr: RefCell<Option<(AddrKind, [u8; 6])>>,
    results: RefCell<Vec<ScanObservation>>,
}

impl HpsScanHandler {
    pub(crate) fn new() -> Self {
        Self {
            found_addr: RefCell::new(None),
            results: RefCell::new(Vec::new()),
        }
    }

    pub(crate) fn prepare_for_scan(&self) {
        if let Ok(mut found) = self.found_addr.try_borrow_mut() {
            *found = None;
        }
        if let Ok(mut results) = self.results.try_borrow_mut() {
            results.clear();
        }
    }

    pub(crate) fn take_found_device(&self) -> Option<(AddrKind, [u8; 6])> {
        self.found_addr.borrow_mut().take()
    }

    pub(crate) fn snapshot_results(&self) -> Vec<ScanObservation> {
        self.results.borrow().clone()
    }

    fn handle_report(&self, report: &LeAdvReport<'_>) {
        let mut addr = [0u8; 6];
        addr.copy_from_slice(report.addr.raw());
        let addr_kind = report.addr_kind;
        let has_hps = has_hps_service(report.data);
        let mut name_opt = extract_device_name(report.data);

        let mut results = self.results.borrow_mut();
        if let Some(existing) = results.iter_mut().find(|entry| entry.addr == addr) {
            existing.addr_kind = addr_kind;
            if let Some(name) = name_opt.take() {
                existing.name = Some(name);
            }
            if has_hps {
                existing.has_hps = true;
            }
        } else {
            results.push(ScanObservation {
                addr,
                addr_kind,
                name: name_opt.take(),
                has_hps,
            });
        }

        if has_hps && self.found_addr.borrow().is_none() {
            info!(
                "HPS: Advertisement from HTTP Proxy candidate at {=[u8]:02X} ({})",
                addr,
                describe_addr_kind(addr_kind)
            );
            *self.found_addr.borrow_mut() = Some((addr_kind, addr));
        }
    }
}

impl EventHandler for HpsScanHandler {
    fn on_adv_reports(&self, mut reports: LeAdvReportsIter<'_>) {
        while let Some(Ok(report)) = reports.next() {
            self.handle_report(&report);
        }
    }
}

pub(crate) fn describe_addr_kind(kind: AddrKind) -> &'static str {
    if kind == AddrKind::PUBLIC {
        "public"
    } else if kind == AddrKind::RANDOM {
        "random"
    } else if kind == AddrKind::RESOLVABLE_PRIVATE_OR_PUBLIC {
        "resolvable/public"
    } else if kind == AddrKind::RESOLVABLE_PRIVATE_OR_RANDOM {
        "resolvable/random"
    } else if kind == AddrKind::ANONYMOUS_ADV {
        "anonymous"
    } else {
        "unknown"
    }
}

fn has_hps_service(ad_data: &[u8]) -> bool {
    let mut offset = 0;
    let target_uuid = HpsUuids::SERVICE.to_le_bytes();

    while offset < ad_data.len() {
        let len = ad_data[offset] as usize;
        if len == 0 {
            break;
        }

        let segment_end = offset + 1 + len;
        if segment_end > ad_data.len() {
            break;
        }

        let ad_type = ad_data[offset + 1];

        if matches!(ad_type, 0x02 | 0x03) {
            let uuid_data = &ad_data[(offset + 2)..segment_end];

            for chunk in uuid_data.chunks(2) {
                if chunk.len() == 2 && chunk == target_uuid {
                    return true;
                }
            }
        }

        offset = segment_end;
    }

    false
}

fn extract_device_name(ad_data: &[u8]) -> Option<String> {
    let mut offset = 0;

    while offset < ad_data.len() {
        let len = ad_data[offset] as usize;
        if len == 0 {
            break;
        }

        let segment_end = offset + 1 + len;
        if segment_end > ad_data.len() {
            break;
        }

        let ad_type = ad_data[offset + 1];
        if matches!(ad_type, 0x08 | 0x09) {
            let name_data = &ad_data[(offset + 2)..segment_end];
            if let Ok(name_str) = core::str::from_utf8(name_data) {
                return Some(String::from(name_str));
            }
        }

        offset = segment_end;
    }

    None
}
