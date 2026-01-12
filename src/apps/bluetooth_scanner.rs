use core::fmt::Write;

use crate::libs::bluetooth::{
    bluetooth, BlePacket, BluetoothEvent, DiscoveredDevice, GattServiceStatus, ScanStatus,
};
use rust_gfx::color::Rgba8888;
use rust_gfx::rasterizer::Rasterizer;
// use rust_gfx::SurfaceDrawTarget; // Removed - requires embedded-graphics
use crate::libs::http_bridge::{HttpBridgeError, HttpClient};
use crate::system::app::app_context::AppContext;
use crate::system::ui::drawing_surface::DrawingSurface;
use defmt::{warn, Debug2Format};
use embassy_executor::task;
use embassy_futures::select::{select3, Either3};
use embassy_time::{Duration, Ticker};
use embedded_graphics::mono_font::ascii::{FONT_6X10, FONT_6X9};
use embedded_graphics::mono_font::MonoTextStyle;
use embedded_graphics::pixelcolor::Rgb888;
use embedded_graphics::prelude::*;
use embedded_graphics::text::Text as EgText;
use heapless::{String, Vec};

const MAX_LISTED_DEVICES: usize = 12;
const HTTP_POLL_INTERVAL_MS: u64 = 30_000;
const USERS_ENDPOINT: &str = "https://jsonplaceholder.typicode.com/users";
const REMOTE_USER_CAPACITY: usize = 3;
const REMOTE_NAME_CAPACITY: usize = 48;
const REMOTE_EMAIL_CAPACITY: usize = 64;
const REMOTE_LINE_CAPACITY: usize = 96;
const HTTP_STATUS_CAPACITY: usize = 64;

type RemoteUserList = Vec<RemoteUser, REMOTE_USER_CAPACITY>;

#[derive(Default)]
struct RemoteUser {
    name: String<REMOTE_NAME_CAPACITY>,
    email: String<REMOTE_EMAIL_CAPACITY>,
}

impl RemoteUser {
    fn set_name(&mut self, value: &str) {
        self.name.clear();
        for ch in value.chars() {
            if !ch.is_ascii() {
                continue;
            }
            if self.name.push(ch).is_err() {
                break;
            }
        }
        if self.name.is_empty() {
            let _ = self.name.push_str("Unknown");
        }
    }

    fn set_email(&mut self, value: &str) {
        self.email.clear();
        for ch in value.chars() {
            if !ch.is_ascii() {
                continue;
            }
            if self.email.push(ch).is_err() {
                break;
            }
        }
    }
}

#[embassy_executor::task]
pub async fn bluetooth_scanner_app(ctx: AppContext) {
    let bt = bluetooth();
    let mut events = bt.events();
    let mut devices: Vec<DiscoveredDevice, MAX_LISTED_DEVICES> = Vec::new();
    let mut status = ScanStatus::Starting;
    let mut gatt_status = GattServiceStatus::Idle;
    let mut last_sent: Option<BlePacket> = None;
    let mut last_received: Option<BlePacket> = None;
    let mut draw_ticker = Ticker::every(Duration::from_millis(500));
    let mut http_ticker = Ticker::every(Duration::from_millis(HTTP_POLL_INTERVAL_MS));
    let mut needs_redraw = true;

    let http = HttpClient::new();
    let mut http_status = String::<HTTP_STATUS_CAPACITY>::new();
    let mut remote_users: RemoteUserList = Vec::new();

    if let Err(e) = bt.start_scan().await {
        warn!("Failed to start BLE scan: {:?}", Debug2Format(&e));
        status = ScanStatus::Failed(e);
        needs_redraw = true;
    }

    match fetch_remote_users(&http).await {
        Ok(users) => {
            remote_users = users;
            http_status.clear();
            let _ = write!(&mut http_status, "Fetched {} user(s)", remote_users.len());
        }
        Err(err) => {
            remote_users.clear();
            http_status.clear();
            let _ = write!(&mut http_status, "Fetch failed: {:?}", err);
        }
    }
    needs_redraw = true;

    loop {
        match select3(events.recv(), draw_ticker.next(), http_ticker.next()).await {
            Either3::First(event) => match event {
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
            },
            Either3::Second(_) => {
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
                        last_sent.as_ref(),
                        last_received.as_ref(),
                        &remote_users,
                        http_status.as_str(),
                    );
                })
                .await;
                needs_redraw = false;
            }
            Either3::Third(_) => {
                match fetch_remote_users(&http).await {
                    Ok(users) => {
                        remote_users = users;
                        http_status.clear();
                        let _ = write!(&mut http_status, "Fetched {} user(s)", remote_users.len());
                    }
                    Err(err) => {
                        remote_users.clear();
                        http_status.clear();
                        let _ = write!(&mut http_status, "Fetch failed: {:?}", err);
                    }
                }
                needs_redraw = true;
            }
        }
    }
}

fn upsert_device(
    devices: &mut Vec<DiscoveredDevice, MAX_LISTED_DEVICES>,
    device: DiscoveredDevice,
) {
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
    last_sent: Option<&BlePacket>,
    last_received: Option<&BlePacket>,
    remote_users: &RemoteUserList,
    http_status: &str,
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

    // TODO: Text rendering disabled - requires embedded-graphics
    // {
    //     let mut target = SurfaceDrawTarget::new(surface);
    //
    //     let _ = EgText::new(
    //         "Bluetooth Scanner",
    //         Point::new(heading_x, heading_y),
    //         title_style,
    //     )
    //     .draw(&mut target);

        let status_y = heading_y + title_height + 2;
        let _ = EgText::new(
            status_text.as_str(),
            Point::new(heading_x, status_y),
            status_style,
        )
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
        let _ = EgText::new(
            status_line.as_str(),
            Point::new(heading_x, cursor_y),
            gatt_style,
        )
        .draw(&mut target);
        cursor_y += line_height;

        let sent_line = format_packet("TX", last_sent);
        let _ = EgText::new(
            sent_line.as_str(),
            Point::new(heading_x, cursor_y),
            entry_style,
        )
        .draw(&mut target);
        cursor_y += line_height;

        let received_line = format_packet("RX", last_received);
        let _ = EgText::new(
            received_line.as_str(),
            Point::new(heading_x, cursor_y),
            entry_style,
        )
        .draw(&mut target);
        cursor_y += line_height;

        cursor_y += line_height;
        let _ = EgText::new("Remote Users", Point::new(heading_x, cursor_y), title_style)
            .draw(&mut target);
        cursor_y += title_height;

        let _ =
            EgText::new(http_status, Point::new(heading_x, cursor_y), gatt_style).draw(&mut target);
        cursor_y += line_height;

        if remote_users.is_empty() {
            let _ = EgText::new(
                "No remote data available",
                Point::new(heading_x, cursor_y),
                placeholder_style,
            )
            .draw(&mut target);
        } else {
            for (idx, user) in remote_users.iter().enumerate() {
                if cursor_y + entry_height >= height - 8 {
                    break;
                }
                let line = format_remote_user_line(idx, user);
                let _ = EgText::new(line.as_str(), Point::new(heading_x, cursor_y), entry_style)
                    .draw(&mut target);
                cursor_y += line_height;
            }
        }
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

fn format_gatt_status(status: GattServiceStatus) -> String<48> {
    let mut s = String::new();
    match status {
        GattServiceStatus::Idle => {
            let _ = s.push_str("Service idle");
        }
        GattServiceStatus::Advertising => {
            let _ = s.push_str("Advertising HTTP bridge");
        }
        GattServiceStatus::Connected => {
            let _ = s.push_str("Central connected");
        }
        GattServiceStatus::SendingRequest => {
            let _ = s.push_str("Sending HTTP request");
        }
        GattServiceStatus::AwaitingResponse => {
            let _ = s.push_str("Awaiting HTTP response");
        }
        GattServiceStatus::ResponseComplete => {
            let _ = s.push_str("Response received");
        }
        GattServiceStatus::Error => {
            let _ = s.push_str("Service error");
        }
    }
    s
}

fn format_packet(prefix: &str, packet: Option<&BlePacket>) -> String<64> {
    let mut line = String::new();
    let _ = write!(&mut line, "{}: ", prefix);
    match packet {
        Some(data) if !data.is_empty() => {
            let _ = write!(&mut line, "{}b ", data.len());
            for &byte in data.iter().take(24) {
                let ch = if (0x20..=0x7E).contains(&byte) {
                    byte as char
                } else {
                    '.'
                };
                let _ = line.push(ch);
            }
        }
        Some(_) => {
            let _ = line.push_str("0b");
        }
        None => {
            let _ = line.push_str("--");
        }
    }
    line
}

fn format_remote_user_line(index: usize, user: &RemoteUser) -> String<REMOTE_LINE_CAPACITY> {
    let mut line = String::new();
    let _ = write!(&mut line, "{:02}. {}", index + 1, user.name.as_str());
    if !user.email.is_empty() {
        let _ = write!(&mut line, " <{}>", user.email.as_str());
    }
    line
}

async fn fetch_remote_users(client: &HttpClient) -> Result<RemoteUserList, HttpBridgeError> {
    let response = client.get(USERS_ENDPOINT).send().await?;
    parse_remote_users(response.body())
}

fn parse_remote_users(body: &[u8]) -> Result<RemoteUserList, HttpBridgeError> {
    let text = core::str::from_utf8(body).map_err(|_| HttpBridgeError::InvalidUtf8)?;
    let mut remainder = text;
    let mut results: RemoteUserList = Vec::new();

    while let Some(start) = remainder.find('{') {
        remainder = &remainder[start + 1..];
        let end = remainder.find('}').ok_or(HttpBridgeError::InvalidJson)?;
        let object = &remainder[..end];

        let name = extract_field(object, "\"name\"").ok_or(HttpBridgeError::InvalidJson)?;
        let email = extract_field(object, "\"email\"").unwrap_or("");

        let mut entry = RemoteUser::default();
        entry.set_name(name);
        entry.set_email(email);
        results.push(entry).ok();
        if results.is_full() {
            break;
        }

        remainder = &remainder[end + 1..];
    }

    Ok(results)
}

fn extract_field<'a>(object: &'a str, key: &str) -> Option<&'a str> {
    let mut search = object;
    while let Some(index) = search.find(key) {
        let after_key_index = index + key.len();
        let after_key = &search[after_key_index..];
        let after_colon = after_key.trim_start();
        if !after_colon.starts_with(':') {
            search = &search[index + 1..];
            continue;
        }
        let after_colon = after_colon[1..].trim_start();
        if !after_colon.starts_with('"') {
            search = &search[index + 1..];
            continue;
        }
        let after_quote = &after_colon[1..];
        if let Some(end) = after_quote.find('"') {
            return Some(&after_quote[..end]);
        } else {
            return None;
        }
    }
    None
}
