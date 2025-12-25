use core::fmt::Write;

use crate::libs::bluetooth::{
    bluetooth,
    BluetoothEvent,
    DiscoveredDevice,
    GattServiceStatus,
    ScanStatus,
};
use crate::libs::gfx::color::Rgba8888;
use crate::libs::gfx::{Rasterizer, SurfaceDrawTarget};
use crate::system::app::app_context::AppContext;
use crate::system::ui::drawing_surface::DrawingSurface;
use defmt::{warn, Debug2Format};
use embassy_futures::select::{select, Either};
use embassy_time::{Duration, Ticker};
use embedded_graphics::mono_font::ascii::{FONT_6X10, FONT_6X9};
use embedded_graphics::mono_font::MonoTextStyle;
use embedded_graphics::pixelcolor::Rgb888;
use embedded_graphics::prelude::*;
use embedded_graphics::text::Text as EgText;
use heapless::{String, Vec};

const MAX_LISTED_DEVICES: usize = 12;

#[embassy_executor::task]
pub async fn bluetooth_scanner_app(ctx: AppContext) {
    let bt = bluetooth();
    let mut events = bt.events();
    let mut devices: Vec<DiscoveredDevice, MAX_LISTED_DEVICES> = Vec::new();
    let mut status = ScanStatus::Starting;
    let mut gatt_status = GattServiceStatus::Idle;
    let mut last_sent: Option<u8> = None;
    let mut last_received: Option<u8> = None;
    let mut ticker = Ticker::every(Duration::from_millis(500));
    let mut needs_redraw = true;

    if let Err(e) = bt.start_scan().await {
        warn!("Failed to start BLE scan: {:?}", Debug2Format(&e));
        status = ScanStatus::Failed(e);
        needs_redraw = true;
    }

    loop {
        match select(events.recv(), ticker.next()).await {
            Either::First(event) => {
                match event {
                    BluetoothEvent::ScanStatus(s) => {
                        status = s;
                        needs_redraw = true;
                    }
                    BluetoothEvent::DeviceDiscovered(device) => {
                        upsert_device(&mut devices, device);
                        needs_redraw = true;
                    }
                    BluetoothEvent::GattServiceStatus(s) => {
                        gatt_status = s;
                        needs_redraw = true;
                    }
                    BluetoothEvent::GattValueSent(value) => {
                        last_sent = Some(value);
                        needs_redraw = true;
                    }
                    BluetoothEvent::GattValueReceived(value) => {
                        last_received = Some(value);
                        needs_redraw = true;
                    }
                }
            }
            Either::Second(_) => {
                if !needs_redraw {
                    continue;
                }
                if !ctx.is_focused().await {
                    continue;
                }
                ctx.draw(|surface: &mut DrawingSurface| {
                    draw_interface(
                        surface,
                        status,
                        &devices,
                        gatt_status,
                        last_sent,
                        last_received,
                    );
                })
                .await;
                needs_redraw = false;
            }
        }
    }
}

fn upsert_device(devices: &mut Vec<DiscoveredDevice, MAX_LISTED_DEVICES>, device: DiscoveredDevice) {
    if let Some(entry) = devices
        .iter_mut()
        .find(|existing| existing.address == device.address)
    {
        if device.name.is_some() {
            entry.name = device.name.clone();
        }
        entry.rssi = device.rssi;
        return;
    }

    if devices.is_full() {
        let _ = devices.pop();
    }
    let _ = devices.push(device);
}

fn draw_interface(
    surface: &mut DrawingSurface,
    status: ScanStatus,
    devices: &Vec<DiscoveredDevice, MAX_LISTED_DEVICES>,
    gatt_status: GattServiceStatus,
    last_sent: Option<u8>,
    last_received: Option<u8>,
) {
    let width = surface.width() as i32;
    let height = surface.height() as i32;
    surface.fill_rect(0, 0, width, height, Rgba8888::rgba(20, 26, 34, 255));

    let title_style = MonoTextStyle::new(&FONT_6X10, Rgb888::new(220, 235, 255));
    let status_style = MonoTextStyle::new(&FONT_6X9, Rgb888::new(150, 195, 255));
    let entry_style = MonoTextStyle::new(&FONT_6X9, Rgb888::new(210, 220, 235));
    let gatt_style = MonoTextStyle::new(&FONT_6X9, Rgb888::new(180, 210, 255));
    let placeholder_style = MonoTextStyle::new(&FONT_6X9, Rgb888::new(120, 140, 160));

    let title_height = title_style.font.character_size.height as i32;
    let status_height = status_style.font.character_size.height as i32;
    let entry_height = entry_style.font.character_size.height as i32;

    let heading_x = 8;
    let heading_y = 14;
    let line_height = entry_height + 2;

    let status_text = format_status(status);

    {
        let mut target = SurfaceDrawTarget::new(surface);

        let _ = EgText::new("Bluetooth Scanner", Point::new(heading_x, heading_y), title_style)
            .draw(&mut target);

        let status_y = heading_y + title_height + 2;
        let _ = EgText::new(status_text.as_str(), Point::new(heading_x, status_y), status_style)
            .draw(&mut target);

        let mut cursor_y = heading_y + title_height + status_height + 4;

        for (idx, device) in devices.iter().enumerate().take(MAX_LISTED_DEVICES) {
            if cursor_y + entry_height >= height - 8 {
                break;
            }
            let line = format_device_line(idx, device);
            let _ = EgText::new(line.as_str(), Point::new(heading_x, cursor_y), entry_style)
                .draw(&mut target);
            cursor_y += line_height;
        }

        if devices.is_empty() {
            let _ = EgText::new(
                "No devices discovered yet",
                Point::new(heading_x, cursor_y),
                placeholder_style,
            )
            .draw(&mut target);
            cursor_y += line_height;
        }

        cursor_y += line_height;
        let _ = EgText::new("GATT Service", Point::new(heading_x, cursor_y), title_style)
            .draw(&mut target);
        cursor_y += title_height;

        let status_line = format_gatt_status(gatt_status);
        let _ = EgText::new(status_line.as_str(), Point::new(heading_x, cursor_y), gatt_style)
            .draw(&mut target);
        cursor_y += line_height;

        let sent_line = format_random_value("TX", last_sent);
        let _ = EgText::new(sent_line.as_str(), Point::new(heading_x, cursor_y), entry_style)
            .draw(&mut target);
        cursor_y += line_height;

        let received_line = format_random_value("RX", last_received);
        let _ = EgText::new(received_line.as_str(), Point::new(heading_x, cursor_y), entry_style)
            .draw(&mut target);
    }
}
fn format_status(status: ScanStatus) -> String<48> {
    let mut s = String::new();
    match status {
        ScanStatus::Idle => {
            let _ = s.push_str("Idle");
        }
        ScanStatus::Starting => {
            let _ = s.push_str("Starting scan...");
        }
        ScanStatus::Running => {
            let _ = s.push_str("Scanning for devices");
        }
        ScanStatus::Stopping => {
            let _ = s.push_str("Stopping scan...");
        }
        ScanStatus::AlreadyRunning => {
            let _ = s.push_str("Scan already running");
        }
        ScanStatus::BlockedByPeripheral => {
            let _ = s.push_str("Scan paused for GATT service");
        }
        ScanStatus::Failed(err) => {
            let _ = write!(&mut s, "Scan failed ({:?})", err);
        }
    }
    s
}

fn format_device_line(index: usize, device: &DiscoveredDevice) -> String<64> {
    let mut line = String::new();
    let _ = write!(&mut line, "{:02}. ", index + 1);

    if let Some(name) = device.name() {
        let _ = line.push_str(name);
    } else {
        let addr = format_address(device.address);
        let _ = line.push_str(addr.as_str());
    }

    let _ = write!(&mut line, " ({:+} dBm)", device.rssi);
    line
}

fn format_address(addr: [u8; 6]) -> String<18> {
    let mut out = String::new();
    for (i, byte) in addr.iter().enumerate() {
        if i != 0 {
            let _ = out.push(':');
        }
        let _ = write!(&mut out, "{:02X}", byte);
    }
    out
}

fn format_gatt_status(status: GattServiceStatus) -> String<32> {
    let mut s = String::new();
    match status {
        GattServiceStatus::Idle => {
            let _ = s.push_str("Service idle");
        }
        GattServiceStatus::Advertising => {
            let _ = s.push_str("Advertising random values");
        }
        GattServiceStatus::Connected => {
            let _ = s.push_str("Central connected");
        }
        GattServiceStatus::Error => {
            let _ = s.push_str("Service error");
        }
    }
    s
}

fn format_random_value(prefix: &str, value: Option<u8>) -> String<32> {
    let mut line = String::new();
    match value {
        Some(v) => {
            let _ = write!(&mut line, "{}: {}", prefix, v);
        }
        None => {
            let _ = write!(&mut line, "{}: --", prefix);
        }
    }
    line
}
