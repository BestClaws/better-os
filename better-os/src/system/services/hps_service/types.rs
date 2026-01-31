use crate::libs::hps::error::HpsError;
use crate::libs::hps::types::{HttpMethod, HttpResponse};
use alloc::string::String;
use alloc::vec::Vec;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::channel::{Channel, Receiver, Sender};

/// HPS request message (heap-allocated to avoid stack overflow)
#[derive(Debug)]
pub struct HpsRequest {
    pub method: HttpMethod,
    pub uri: String,
    pub headers: String,
    pub body: Vec<u8>,
}

/// HPS response message
pub type HpsResponse = Result<HttpResponse, HpsError>;

/// Channel for HPS requests
static HPS_REQUEST_CHANNEL: Channel<CriticalSectionRawMutex, HpsRequest, 2> = Channel::new();

/// Channel for HPS responses
static HPS_RESPONSE_CHANNEL: Channel<CriticalSectionRawMutex, HpsResponse, 2> = Channel::new();

/// Channel for HPS ready signal - sent when connection is established
static HPS_READY_CHANNEL: Channel<CriticalSectionRawMutex, bool, 1> = Channel::new();

/// Get sender for HPS requests (for apps to use)
pub fn hps_request_sender() -> Sender<'static, CriticalSectionRawMutex, HpsRequest, 2> {
    HPS_REQUEST_CHANNEL.sender()
}

/// Get receiver for HPS responses (for apps to use)
pub fn hps_response_receiver() -> Receiver<'static, CriticalSectionRawMutex, HpsResponse, 2> {
    HPS_RESPONSE_CHANNEL.receiver()
}

/// Wait for HPS service to be ready
/// Returns immediately if already ready, otherwise blocks until ready signal is sent
pub async fn wait_for_hps_ready() {
    HPS_READY_CHANNEL.receiver().receive().await;
}

pub(crate) fn request_receiver() -> Receiver<'static, CriticalSectionRawMutex, HpsRequest, 2> {
    HPS_REQUEST_CHANNEL.receiver()
}

pub(crate) fn response_sender() -> Sender<'static, CriticalSectionRawMutex, HpsResponse, 2> {
    HPS_RESPONSE_CHANNEL.sender()
}

pub(crate) fn signal_ready() {
    let _ = HPS_READY_CHANNEL.sender().try_send(true);
}
