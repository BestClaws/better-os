use core::sync::atomic::{AtomicBool, Ordering};

use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::channel::{Channel, Receiver, Sender, TryReceiveError};
use embassy_sync::mutex::Mutex;
use embassy_sync::signal::Signal;
use heapless::{String, Vec};

use crate::libs::bluetooth::BLE_PACKET_CAPACITY;

const REQUEST_QUEUE_DEPTH: usize = 1;
const DEFAULT_HOST: &str = "ble-http.local";

/// Maximum supported HTTP request size in bytes.
pub const MAX_HTTP_REQUEST: usize = 256;
/// Maximum supported HTTP response size in bytes (headers + body).
pub const MAX_HTTP_RESPONSE: usize = 512;
/// Maximum number of bytes returned in the HTTP response body.
pub const MAX_HTTP_BODY: usize = 256;

/// Buffer used to transport HTTP requests to the Bluetooth service.
pub type HttpRequestBuffer = Vec<u8, MAX_HTTP_REQUEST>;
/// Buffer used to transport complete HTTP responses from the Bluetooth service.
pub type HttpResponseBuffer = Vec<u8, MAX_HTTP_RESPONSE>;
/// Buffer containing just the HTTP response body.
pub type HttpBodyBuffer = Vec<u8, MAX_HTTP_BODY>;

/// Result type delivered back to HTTP bridge callers.
pub type HttpBridgeResult = Result<HttpResponseBuffer, HttpBridgeError>;

/// Receiver handed to the Bluetooth service for processing outbound requests.
pub type HttpRequestReceiver =
    Receiver<'static, CriticalSectionRawMutex, HttpRequestBuffer, REQUEST_QUEUE_DEPTH>;
/// Sender available to application code for enqueueing outbound requests.
pub type HttpRequestSender =
    Sender<'static, CriticalSectionRawMutex, HttpRequestBuffer, REQUEST_QUEUE_DEPTH>;
/// Signal used to return responses (or errors) to awaiting application code.
pub type HttpResponseSignal = Signal<CriticalSectionRawMutex, HttpBridgeResult>;

static REQUEST_CHANNEL: Channel<CriticalSectionRawMutex, HttpRequestBuffer, REQUEST_QUEUE_DEPTH> =
    Channel::new();
static RESPONSE_SIGNAL: HttpResponseSignal = Signal::new();
static REQUEST_LOCK: Mutex<CriticalSectionRawMutex, ()> = Mutex::new(());
static CONNECTED: AtomicBool = AtomicBool::new(false);
static INFLIGHT: AtomicBool = AtomicBool::new(false);

/// Error conditions surfaced by the HTTP bridge client.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HttpBridgeError {
    RequestTooLarge,
    ResponseTooLarge,
    InvalidUtf8,
    InvalidResponse,
    NoConnection,
    Disconnected,
    Internal,
}

impl defmt::Format for HttpBridgeError {
    fn format(&self, f: defmt::Formatter) {
        let label = match self {
            Self::RequestTooLarge => "RequestTooLarge",
            Self::ResponseTooLarge => "ResponseTooLarge",
            Self::InvalidUtf8 => "InvalidUtf8",
            Self::InvalidResponse => "InvalidResponse",
            Self::NoConnection => "NoConnection",
            Self::Disconnected => "Disconnected",
            Self::Internal => "Internal",
        };
        defmt::write!(f, "HttpBridgeError::{}", label);
    }
}

/// Parsed HTTP response returned to callers.
pub struct HttpResponse {
    pub status: u16,
    pub body: HttpBodyBuffer,
}

/// Obtain a sender for posting HTTP requests.
pub fn request_sender() -> HttpRequestSender {
    REQUEST_CHANNEL.sender()
}

/// Obtain the receiver consumed by the Bluetooth service.
pub(crate) fn request_receiver() -> HttpRequestReceiver {
    REQUEST_CHANNEL.receiver()
}

/// Access the signal used to deliver responses back to callers.
pub(crate) fn response_signal() -> &'static HttpResponseSignal {
    &RESPONSE_SIGNAL
}

/// Notify the bridge of the current connection state.
pub(crate) fn set_connection_state(connected: bool) {
    CONNECTED.store(connected, Ordering::SeqCst);
}

/// Returns `true` if the HTTP bridge currently has an in-flight request.
pub(crate) fn has_inflight_request() -> bool {
    INFLIGHT.load(Ordering::Acquire)
}

/// Notify the bridge that a response (success) is ready.
pub(crate) fn deliver_response(response: HttpResponseBuffer) {
    if INFLIGHT.swap(false, Ordering::SeqCst) {
        RESPONSE_SIGNAL.signal(Ok(response));
    }
}

/// Notify the bridge that the current request failed.
pub(crate) fn deliver_error(error: HttpBridgeError) {
    if INFLIGHT.swap(false, Ordering::SeqCst) {
        RESPONSE_SIGNAL.signal(Err(error));
    }
}

/// Drop any queued but unprocessed requests, returning whether a request was discarded.
pub(crate) fn discard_pending_request(receiver: &mut HttpRequestReceiver) -> bool {
    match receiver.try_receive() {
        Ok(_) => true,
        Err(_) => false,
    }
}

/// Submit a `GET` request to the BLE HTTP bridge and return the parsed response.
pub async fn send_get(path: &str) -> Result<HttpResponse, HttpBridgeError> {
    let mut request = String::<MAX_HTTP_REQUEST>::new();

    if !path.starts_with('/') {
        request
            .push_str("GET /")
            .map_err(|_| HttpBridgeError::RequestTooLarge)?;
        request
            .push_str(path)
            .map_err(|_| HttpBridgeError::RequestTooLarge)?;
    } else {
        request
            .push_str("GET ")
            .map_err(|_| HttpBridgeError::RequestTooLarge)?;
        request
            .push_str(path)
            .map_err(|_| HttpBridgeError::RequestTooLarge)?;
    }

    request
        .push_str(" HTTP/1.1\r\nHost: ")
        .map_err(|_| HttpBridgeError::RequestTooLarge)?;
    request
        .push_str(DEFAULT_HOST)
        .map_err(|_| HttpBridgeError::RequestTooLarge)?;
    request
        .push_str("\r\nConnection: close\r\n\r\n")
        .map_err(|_| HttpBridgeError::RequestTooLarge)?;

    let raw = send_raw_request(request.as_bytes()).await?;
    parse_http_response(raw.as_slice())
}

/// Core helper for dispatching raw HTTP requests across the BLE bridge.
pub async fn send_raw_request(request_bytes: &[u8]) -> Result<HttpResponseBuffer, HttpBridgeError> {
    if request_bytes.len() > MAX_HTTP_REQUEST || request_bytes.len() > BLE_PACKET_CAPACITY * 16 {
        return Err(HttpBridgeError::RequestTooLarge);
    }

    if !CONNECTED.load(Ordering::Acquire) {
        return Err(HttpBridgeError::NoConnection);
    }

    let _guard = REQUEST_LOCK.lock().await;

    if !CONNECTED.load(Ordering::Acquire) {
        return Err(HttpBridgeError::NoConnection);
    }

    let mut request = HttpRequestBuffer::new();
    request
        .extend_from_slice(request_bytes)
        .map_err(|_| HttpBridgeError::RequestTooLarge)?;

    INFLIGHT.store(true, Ordering::SeqCst);
    request_sender().send(request).await;

    match RESPONSE_SIGNAL.wait().await {
        Ok(response) => Ok(response),
        Err(error) => Err(error),
    }
}

fn parse_http_response(raw: &[u8]) -> Result<HttpResponse, HttpBridgeError> {
    let text = core::str::from_utf8(raw).map_err(|_| HttpBridgeError::InvalidUtf8)?;
    let header_end = text
        .find("\r\n\r\n")
        .ok_or(HttpBridgeError::InvalidResponse)?;

    let (header_part, body_part) = text.split_at(header_end);
    let body_bytes = raw.get(header_end + 4..).unwrap_or(&[]);

    let mut lines = header_part.lines();
    let status_line = lines.next().ok_or(HttpBridgeError::InvalidResponse)?;
    let status = parse_status_code(status_line).ok_or(HttpBridgeError::InvalidResponse)?;

    if body_bytes.len() > MAX_HTTP_BODY {
        return Err(HttpBridgeError::ResponseTooLarge);
    }

    let mut body = HttpBodyBuffer::new();
    body.extend_from_slice(body_bytes)
        .map_err(|_| HttpBridgeError::ResponseTooLarge)?;

    Ok(HttpResponse { status, body })
}

fn parse_status_code(line: &str) -> Option<u16> {
    let mut parts = line.split_whitespace();
    let _ = parts.next()?;
    let code_str = parts.next()?;
    code_str.parse().ok()
}
