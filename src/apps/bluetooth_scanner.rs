use core::fmt::Write;

use crate::libs::bluetooth::{bluetooth, BluetoothEvent, DiscoveredDevice, ScanStatus};
use crate::libs::gfx::color::Rgba8888;
use crate::libs::gfx::font::{font_for_size, FontSize, DEFAULT_CHARSETS};
use crate::libs::gfx::shapes::{Shape, Text};
use crate::libs::gfx::Rasterizer;
use crate::system::app::app_context::AppContext;
use crate::system::ui::drawing_surface::DrawingSurface;
use defmt::{warn, Debug2Format};
use embassy_futures::select::{select, Either};
use embassy_time::{Duration, Ticker};
use heapless::{String, Vec};

const MAX_LISTED_DEVICES: usize = 12;

#[embassy_executor::task]
pub async fn bluetooth_scanner_app(ctx: AppContext) {
    let bt = bluetooth();
    let mut events = bt.events();
    let mut devices: Vec<DiscoveredDevice, MAX_LISTED_DEVICES> = Vec::new();
    let mut status = ScanStatus::Starting;
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
                    draw_interface(surface, status, &devices);
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

fn draw_interface(surface: &mut DrawingSurface, status: ScanStatus, devices: &Vec<DiscoveredDevice, MAX_LISTED_DEVICES>) {
    let width = surface.width() as i32;
    let height = surface.height() as i32;
    surface.fill_rect(0, 0, width, height, Rgba8888::rgba(20, 26, 34, 255));

    let title_font = font_for_size(FontSize::Medium).with_charsets(DEFAULT_CHARSETS);
    let status_font = font_for_size(FontSize::Small).with_charsets(DEFAULT_CHARSETS);
    let entry_font = font_for_size(FontSize::Small).with_charsets(DEFAULT_CHARSETS);

    Text::new(16, 24, "Bluetooth Scanner")
        .font(title_font)
        .color(Rgba8888::rgba(220, 235, 255, 255))
        .draw(surface);

    let status_text = format_status(status);
    Text::new(16, 24 + title_font.line_advance(), status_text.as_str())
        .font(status_font)
        .color(Rgba8888::rgba(150, 195, 255, 220))
        .draw(surface);

    let mut cursor_y = 24 + title_font.line_advance() + status_font.line_advance() + 10;
    let line_height = entry_font.line_advance();

    for (idx, device) in devices.iter().enumerate().take(MAX_LISTED_DEVICES) {
        if cursor_y + line_height >= height - 16 {
            break;
        }
        let line = format_device_line(idx, device);
        Text::new(16, cursor_y, line.as_str())
            .font(entry_font)
            .color(Rgba8888::rgba(210, 220, 235, 240))
            .draw(surface);
        cursor_y += line_height;
    }

    if devices.is_empty() {
        Text::new(16, cursor_y, "No devices discovered yet")
            .font(entry_font)
            .color(Rgba8888::rgba(120, 140, 160, 200))
            .draw(surface);
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
