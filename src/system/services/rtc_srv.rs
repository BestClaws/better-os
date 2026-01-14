use crate::system::hal::rtc::{AsyncRtc, RtcDateTime, RtcError};
use alloc::boxed::Box;
use defmt::{info, warn, Format};
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::channel::Channel;
use embassy_sync::mutex::Mutex;
use embassy_sync::signal::Signal;
use embassy_time::{Duration, Timer};

static RTC_STATE: Mutex<CriticalSectionRawMutex, Option<RtcDateTime>> = Mutex::new(None);
static RTC_READY: Signal<CriticalSectionRawMutex, ()> = Signal::new();
static RTC_COMMANDS: Channel<CriticalSectionRawMutex, RtcCommand, 2> = Channel::new();

const DEFAULT_BOOT_DATETIME: RtcDateTime = RtcDateTime::new(2026, 1, 1, 0, 0, 0, 0);

#[derive(Clone, Copy, Debug, Format, PartialEq, Eq)]
pub enum RtcServiceError {
    Parse,
}

#[derive(Clone, Copy)]
enum RtcCommand {
    Set(RtcDateTime),
}

/// Returns the most recently sampled RTC value, if available.
pub async fn current_datetime() -> Option<RtcDateTime> {
    let state = RTC_STATE.lock().await;
    *state
}

pub async fn set_datetime(datetime: &RtcDateTime) -> Result<(), RtcServiceError> {
    RTC_READY.wait().await;
    RTC_COMMANDS.sender().send(RtcCommand::Set(*datetime)).await;
    Ok(())
}

pub async fn set_datetime_from_worldtime(payload: &[u8]) -> Result<(), RtcServiceError> {
    let datetime = parse_worldtime_payload(payload).map_err(|err| {
        warn!("WorldTime payload parse failed: {:?}", err);
        RtcServiceError::Parse
    })?;
    set_datetime(&datetime).await
}

#[embassy_executor::task]
pub(crate) async fn rtc_service(rtc: &'static Mutex<CriticalSectionRawMutex, Box<dyn AsyncRtc>>) {
    {
        let mut guard = rtc.lock().await;
        if let Err(err) = guard.init().await {
            warn!("RTC init failed: {:?}", err);
        }
    }

    RTC_READY.signal(());

    let mut integrity_warned = false;
    let mut recovery_attempts: u8 = 0;

    loop {
        let sample = {
            let mut guard = rtc.lock().await;

            handle_pending_commands(&mut guard).await;

            match guard.now().await {
                Ok(value) => Some(value),
                Err(err) => {
                    match err {
                        RtcError::InvalidData => {
                            if !integrity_warned {
                                warn!("RTC clock integrity lost; applying default timestamp");
                                integrity_warned = true;
                            }

                            if recovery_attempts < 5 {
                                recovery_attempts += 1;
                                if let Err(write_err) = guard.set(&DEFAULT_BOOT_DATETIME).await {
                                    warn!("RTC recovery write failed: {:?}", write_err);
                                } else {
                                    info!("RTC default time written");
                                }
                            }
                        }
                        other => warn!("RTC read failed: {:?}", other),
                    }
                    None
                }
            }
        };

        if let Some(value) = sample {
            if integrity_warned {
                info!(
                    "RTC recovered; now {:?}-{:?}-{:?} {:?}:{:?}:{:?}",
                    value.year, value.month, value.day, value.hour, value.minute, value.second
                );
            }
            integrity_warned = false;
            recovery_attempts = 0;
            let mut state = RTC_STATE.lock().await;
            *state = Some(value);
        }

        Timer::after(Duration::from_secs(1)).await;
    }
}

async fn handle_pending_commands(rtc: &mut Box<dyn AsyncRtc>) {
    while let Ok(command) = RTC_COMMANDS.try_receive() {
        match command {
            RtcCommand::Set(datetime) => match rtc.set(&datetime).await {
                Ok(()) => {
                    info!(
                        "RTC manually set to {:04}-{:02}-{:02} {:02}:{:02}:{:02}",
                        datetime.year,
                        datetime.month,
                        datetime.day,
                        datetime.hour,
                        datetime.minute,
                        datetime.second
                    );
                    let mut state = RTC_STATE.lock().await;
                    *state = Some(datetime);
                }
                Err(err) => {
                    warn!("RTC set command failed: {:?}", err);
                }
            },
        }
    }
}

#[derive(Debug, Format)]
enum WorldTimeParseError {
    Utf8,
    Datetime,
    DayOfWeek,
}

fn parse_worldtime_payload(payload: &[u8]) -> Result<RtcDateTime, WorldTimeParseError> {
    let text = core::str::from_utf8(payload).map_err(|_| WorldTimeParseError::Utf8)?;

    let datetime_str =
        extract_string_field(text, "\"datetime\":\"").ok_or(WorldTimeParseError::Datetime)?;
    let day_of_week =
        extract_numeric_field(text, "\"day_of_week\":").ok_or(WorldTimeParseError::DayOfWeek)?;

    parse_worldtime_datetime(datetime_str, day_of_week).ok_or(WorldTimeParseError::Datetime)
}

fn extract_string_field<'a>(text: &'a str, key: &str) -> Option<&'a str> {
    let start = text.find(key)? + key.len();
    let remainder = &text[start..];
    let end = remainder.find('"')?;
    Some(&remainder[..end])
}

fn extract_numeric_field(text: &str, key: &str) -> Option<u8> {
    let start = text.find(key)? + key.len();
    let remainder = &text[start..];
    let digits_end = remainder
        .char_indices()
        .find(|(_, ch)| !ch.is_ascii_digit())
        .map(|(idx, _)| idx)
        .unwrap_or_else(|| remainder.len());
    let digits = &remainder[..digits_end];
    if digits.is_empty() {
        return None;
    }
    digits.parse().ok()
}

fn parse_worldtime_datetime(datetime: &str, day_of_week: u8) -> Option<RtcDateTime> {
    if datetime.len() < 19 {
        return None;
    }

    let year = datetime.get(0..4)?.parse().ok()?;
    let month = datetime.get(5..7)?.parse().ok()?;
    let day = datetime.get(8..10)?.parse().ok()?;
    let hour = datetime.get(11..13)?.parse().ok()?;
    let minute = datetime.get(14..16)?.parse().ok()?;
    let second = datetime.get(17..19)?.parse().ok()?;

    if !(1..=12).contains(&month)
        || !(1..=31).contains(&day)
        || hour > 23
        || minute > 59
        || second > 59
    {
        return None;
    }

    let weekday = match day_of_week {
        0 => 0,
        n => n % 7,
    };

    Some(RtcDateTime::new(
        year, month, day, hour, minute, second, weekday,
    ))
}
