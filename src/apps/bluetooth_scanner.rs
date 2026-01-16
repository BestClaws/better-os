use alloc::{format, string::String, vec, vec::Vec};
use core::fmt::Write;

use crate::apps::components::{
    draw_background, draw_card, draw_list, draw_status_bar, AccentColor, BadgeConfig, BadgeTone,
    CardConfig, StatusBarData, StyleFonts, StyleMetrics, StylePalette,
};
use crate::libs::bluetooth::{
    bluetooth, BlePacket, BluetoothEvent, DiscoveredDevice, GattServiceStatus, ScanStatus,
};
use crate::libs::http_bridge::{HttpBridgeError, HttpClient};
use crate::system::app::app_context::AppContext;
use crate::system::ui::drawing_surface::DrawingSurface;
use defmt::{warn, Debug2Format};
use embassy_executor::task;
use embassy_futures::select::{select3, Either3};
use embassy_time::{Duration, Ticker};

const MAX_LISTED_DEVICES: usize = 12;
const HTTP_POLL_INTERVAL_MS: u64 = 30_000;
const USERS_ENDPOINT: &str = "https://jsonplaceholder.typicode.com/users";
const REMOTE_USER_CAPACITY: usize = 3;
const REMOTE_NAME_CAPACITY: usize = 48;
const REMOTE_EMAIL_CAPACITY: usize = 64;
const REMOTE_LINE_CAPACITY: usize = 96;
const HTTP_STATUS_CAPACITY: usize = 64;
const MAX_DEVICE_DISPLAY: usize = 6;

type RemoteUserList = Vec<RemoteUser>;

#[derive(Default)]
struct RemoteUser {
    name: String,
    email: String,
}

fn status_badge_text(status: ScanStatus) -> &'static str {
    match status {
        ScanStatus::Starting => "STARTING",
        ScanStatus::Running => "SCANNING",
        ScanStatus::Stopping => "STOPPING",
        ScanStatus::BlockedByPeripheral => "PAUSED",
        ScanStatus::AlreadyRunning => "ACTIVE",
        ScanStatus::Failed(_) => "FAILED",
        ScanStatus::Idle => "IDLE",
    }
}

fn status_badge_tone(status: ScanStatus) -> BadgeTone {
    match status {
        ScanStatus::Failed(_) => BadgeTone::Danger,
        ScanStatus::Idle => BadgeTone::Gray,
        _ => BadgeTone::Accent,
    }
}

fn list_card_height(line_count: usize, fonts: StyleFonts, metrics: &StyleMetrics) -> i32 {
    if line_count == 0 {
        return metrics.section_padding * 2 + fonts.line_height_title();
    }
    let lines = line_count as i32 * fonts.line_height_body();
    let dividers = line_count.saturating_sub(1) as i32 * (metrics.section_padding / 2);
    metrics.section_padding * 2 + fonts.line_height_title() + metrics.section_padding / 2 + lines + dividers
}

fn adjust_card_height(desired: i32, min: i32, max: i32) -> i32 {
    if max <= 0 {
        return 0;
    }
    if max < min {
        max
    } else {
        desired.clamp(min, max)
    }
}

fn build_device_lines(status: ScanStatus, devices: &[DiscoveredDevice]) -> Vec<String> {
    let mut lines = Vec::with_capacity(MAX_DEVICE_DISPLAY + 2);
    lines.push(format_status(status));

    for (idx, device) in devices.iter().enumerate().take(MAX_DEVICE_DISPLAY) {
        lines.push(format_device_line(idx, device));
    }

    if devices.is_empty() {
        lines.push("No devices discovered".to_string());
    } else if devices.len() > MAX_DEVICE_DISPLAY {
        let remaining = devices.len() - MAX_DEVICE_DISPLAY;
        lines.push(format!("+{} more device(s)", remaining));
    }

    lines
}

fn build_gatt_lines(
    status: GattServiceStatus,
    last_sent: Option<&BlePacket>,
    last_received: Option<&BlePacket>,
) -> Vec<String> {
    vec![
        format_gatt_status(status),
        format_packet("TX", last_sent),
        format_packet("RX", last_received),
    ]
}

fn build_remote_lines(http_status: &str, remote_users: &RemoteUserList) -> Vec<String> {
    let mut lines = Vec::with_capacity(remote_users.len() + 2);
    lines.push(clamp_ascii(http_status, REMOTE_LINE_CAPACITY));

    if remote_users.is_empty() {
        lines.push("No remote data available".to_string());
        return lines;
    }

    for (idx, user) in remote_users.iter().enumerate() {
        let line = format_remote_user_line(idx, user);
        lines.push(clamp_ascii(line.as_str(), REMOTE_LINE_CAPACITY));
    }

    lines
}

fn clamp_ascii(input: &str, max_len: usize) -> String {
    let mut out = String::with_capacity(max_len.min(input.len()));
    for ch in input.chars() {
        if !ch.is_ascii() {
            continue;
        }
        if out.len() >= max_len {
            break;
        }
        out.push(ch);
    }
    if out.is_empty() {
        out.push('-');
    }
    out
}

impl RemoteUser {
    fn set_name(&mut self, value: &str) {
        self.name.clear();
        for ch in value.chars() {
            if !ch.is_ascii() {
                continue;
            }
            if self.name.len() >= REMOTE_NAME_CAPACITY {
                break;
            }
            self.name.push(ch);
        }
        if self.name.is_empty() {
            self.name.push_str("Unknown");
        }
    }

    fn set_email(&mut self, value: &str) {
        self.email.clear();
        for ch in value.chars() {
            if !ch.is_ascii() {
                continue;
            }
            if self.email.len() >= REMOTE_EMAIL_CAPACITY {
                break;
            }
            self.email.push(ch);
        }
    }
}

#[embassy_executor::task]
pub async fn bluetooth_scanner_app(ctx: AppContext) {
    let bt = bluetooth();
    let mut events = bt.events();
    let mut devices: Vec<DiscoveredDevice> = Vec::with_capacity(MAX_LISTED_DEVICES);
    let mut status = ScanStatus::Starting;
    let mut gatt_status = GattServiceStatus::Idle;
    let mut last_sent: Option<BlePacket> = None;
    let mut last_received: Option<BlePacket> = None;
    let mut draw_ticker = Ticker::every(Duration::from_millis(500));
    let mut http_ticker = Ticker::every(Duration::from_millis(HTTP_POLL_INTERVAL_MS));
    let mut needs_redraw = true;

    let http = HttpClient::new();
    let mut http_status = String::with_capacity(HTTP_STATUS_CAPACITY);
    let mut remote_users: RemoteUserList = Vec::with_capacity(REMOTE_USER_CAPACITY);

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
    devices: &mut Vec<DiscoveredDevice>,
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

    if devices.len() >= MAX_LISTED_DEVICES {
        devices.remove(0);
    }
    devices.push(device);
}

fn draw_interface(
    surface: &mut DrawingSurface,
    status: ScanStatus,
    devices: &[DiscoveredDevice],
    gatt_status: GattServiceStatus,
    last_sent: Option<&BlePacket>,
    last_received: Option<&BlePacket>,
    remote_users: &RemoteUserList,
    http_status: &str,
) {
    let metrics = StyleMetrics::from_surface(surface);
    let palette = StylePalette::arknights();
    let fonts = StyleFonts::for_surface(metrics.width, metrics.height);

    draw_background(surface, &metrics, palette);

    let status_badge = status_badge_text(status);
    let status_area = draw_status_bar(
        surface,
        &metrics,
        fonts,
        palette,
        StatusBarData {
            left: "Bluetooth",
            battery_percent: 66,
            right: Some(status_badge),
        },
    );

    let mut next_y = status_area.y2 + 1 + metrics.section_spacing;
    let mut available = metrics.height.saturating_sub(next_y);
    if available <= metrics.section_padding {
        return;
    }

    let device_lines = build_device_lines(status, devices);
    let device_refs: Vec<&str> = device_lines.iter().map(|line| line.as_str()).collect();
    if !device_refs.is_empty() {
        let desired = list_card_height(device_refs.len(), fonts, &metrics);
        let height = adjust_card_height(desired, metrics.button_height * 2, available);
        if height > 0 {
            let device_card = draw_card(
                surface,
                &metrics,
                fonts,
                palette,
                next_y,
                CardConfig {
                    title: Some("Nearby Devices"),
                    subtitle: Some("Scan results"),
                    badge: Some(BadgeConfig {
                        text: status_badge,
                        tone: status_badge_tone(status),
                    }),
                    accent: AccentColor::Yellow,
                    height,
                },
            );
            draw_list(surface, &device_card, fonts, palette, &device_refs);

            next_y = device_card.next_y(&metrics);
            available = metrics.height.saturating_sub(next_y);
        }
    }

    if available <= metrics.section_padding {
        return;
    }

    let gatt_lines = build_gatt_lines(gatt_status, last_sent, last_received);
    let gatt_refs: Vec<&str> = gatt_lines.iter().map(|line| line.as_str()).collect();
    if !gatt_refs.is_empty() {
        let desired = list_card_height(gatt_refs.len(), fonts, &metrics);
        let height = adjust_card_height(desired, metrics.button_height * 2, available);
        if height > 0 {
            let gatt_card = draw_card(
                surface,
                &metrics,
                fonts,
                palette,
                next_y,
                CardConfig {
                    title: Some("GATT Bridge"),
                    subtitle: Some("HTTP proxy status"),
                    badge: None,
                    accent: AccentColor::Gray,
                    height,
                },
            );
            draw_list(surface, &gatt_card, fonts, palette, &gatt_refs);

            next_y = gatt_card.next_y(&metrics);
            available = metrics.height.saturating_sub(next_y);
        }
    }

    if available <= metrics.section_padding {
        return;
    }

    let remote_lines = build_remote_lines(http_status, remote_users);
    let remote_refs: Vec<&str> = remote_lines.iter().map(|line| line.as_str()).collect();
    if remote_refs.is_empty() {
        return;
    }

    let desired = list_card_height(remote_refs.len(), fonts, &metrics);
    let height = adjust_card_height(desired, metrics.button_height * 2, available);
    if height <= 0 {
        return;
    }

    let remote_card = draw_card(
        surface,
        &metrics,
        fonts,
        palette,
        next_y,
        CardConfig {
            title: Some("Remote Users"),
            subtitle: Some("Last HTTP sync"),
            badge: Some(BadgeConfig {
                text: if remote_users.is_empty() { "EMPTY" } else { "SYNC" },
                tone: if remote_users.is_empty() {
                    BadgeTone::Gray
                } else {
                    BadgeTone::Accent
                },
            }),
            accent: AccentColor::Gray,
            height,
        },
    );

    draw_list(surface, &remote_card, fonts, palette, &remote_refs);
}

fn format_status(status: ScanStatus) -> String {
    let mut s = String::with_capacity(48);
    match status {
        ScanStatus::Idle => {
            s.push_str("Idle");
        }
        ScanStatus::Starting => {
            s.push_str("Starting scan...");
        }
        ScanStatus::Running => {
            s.push_str("Scanning for devices");
        }
        ScanStatus::Stopping => {
            s.push_str("Stopping scan...");
        }
        ScanStatus::AlreadyRunning => {
            s.push_str("Scan already running");
        }
        ScanStatus::BlockedByPeripheral => {
            s.push_str("Scan paused for GATT service");
        }
        ScanStatus::Failed(err) => {
            let _ = write!(&mut s, "Scan failed ({:?})", err);
        }
    }
    s
}

fn format_device_line(index: usize, device: &DiscoveredDevice) -> String {
    let mut line = String::with_capacity(64);
    let _ = write!(&mut line, "{:02}. ", index + 1);

    if let Some(name) = device.name() {
        line.push_str(name);
    } else {
        let addr = format_address(device.address);
        line.push_str(addr.as_str());
    }

    let _ = write!(&mut line, " ({:+} dBm)", device.rssi);
    line
}

fn format_address(addr: [u8; 6]) -> String {
    let mut out = String::with_capacity(18);
    for (i, byte) in addr.iter().enumerate() {
        if i != 0 {
            out.push(':');
        }
        let _ = write!(&mut out, "{:02X}", byte);
    }
    out
}

fn format_gatt_status(status: GattServiceStatus) -> String {
    let mut s = String::with_capacity(48);
    match status {
        GattServiceStatus::Idle => {
            s.push_str("Service idle");
        }
        GattServiceStatus::Advertising => {
            s.push_str("Advertising HTTP bridge");
        }
        GattServiceStatus::Connected => {
            s.push_str("Central connected");
        }
        GattServiceStatus::SendingRequest => {
            s.push_str("Sending HTTP request");
        }
        GattServiceStatus::AwaitingResponse => {
            s.push_str("Awaiting HTTP response");
        }
        GattServiceStatus::ResponseComplete => {
            s.push_str("Response received");
        }
        GattServiceStatus::Error => {
            s.push_str("Service error");
        }
    }
    s
}

fn format_packet(prefix: &str, packet: Option<&BlePacket>) -> String {
    let mut line = String::with_capacity(64);
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
                line.push(ch);
            }
        }
        Some(_) => {
            line.push_str("0b");
        }
        None => {
            line.push_str("--");
        }
    }
    line
}

fn format_remote_user_line(index: usize, user: &RemoteUser) -> String {
    let mut line = String::with_capacity(REMOTE_LINE_CAPACITY);
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

        if results.len() >= REMOTE_USER_CAPACITY {
            break;
        }

        let mut entry = RemoteUser {
            name: String::with_capacity(REMOTE_NAME_CAPACITY),
            email: String::with_capacity(REMOTE_EMAIL_CAPACITY),
        };
        entry.set_name(name);
        entry.set_email(email);
        results.push(entry);

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
