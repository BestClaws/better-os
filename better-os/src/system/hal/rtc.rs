use alloc::boxed::Box;
use async_trait::async_trait;
use defmt::Format;

/// Calendar date-time as provided by hardware RTCs.
#[derive(Clone, Copy, Debug, Format, PartialEq, Eq)]
pub struct RtcDateTime {
    pub year: u16,
    pub month: u8,
    pub day: u8,
    pub hour: u8,
    pub minute: u8,
    pub second: u8,
    pub weekday: u8,
}

impl RtcDateTime {
    pub const fn new(
        year: u16,
        month: u8,
        day: u8,
        hour: u8,
        minute: u8,
        second: u8,
        weekday: u8,
    ) -> Self {
        Self {
            year,
            month,
            day,
            hour,
            minute,
            second,
            weekday,
        }
    }
}

/// Errors produced by RTC drivers.
#[derive(Clone, Copy, Debug, Format, PartialEq, Eq)]
pub enum RtcError {
    /// Underlying peripheral communication failed.
    Bus,
    /// RTC reported invalid or corrupted data (e.g. clock integrity flag).
    InvalidData,
}

#[async_trait(?Send)]
pub trait AsyncRtc {
    async fn init(&mut self) -> Result<(), RtcError>;
    async fn now(&mut self) -> Result<RtcDateTime, RtcError>;
    async fn set(&mut self, datetime: &RtcDateTime) -> Result<(), RtcError>;
}
