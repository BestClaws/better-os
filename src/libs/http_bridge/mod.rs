use core::fmt::Write;
use core::sync::atomic::{AtomicBool, Ordering};

use alloc::vec::Vec;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::channel::{Channel, Receiver, Sender, TryReceiveError};
use embassy_sync::mutex::Mutex;
use embassy_sync::signal::Signal;
use heapless::{String, Vec as HeaplessVec};
use httparse::{Response as HttpParseResponse, Status, EMPTY_HEADER};
use serde::de::DeserializeOwned;
use serde_json_core as json;

use crate::libs::bluetooth::BLE_PACKET_CAPACITY;

pub mod gatt;

const REQUEST_QUEUE_DEPTH: usize = 1;
const DEFAULT_HOST: &str = "ble-http.local";

/// Maximum supported HTTP request size in bytes.
pub const MAX_HTTP_REQUEST: usize = 2048;
/// Maximum supported HTTP response size in bytes (headers + body).
pub const MAX_HTTP_RESPONSE: usize = 10 * 1024;
/// Maximum number of bytes returned in the HTTP response body.
pub const MAX_HTTP_BODY: usize = 10 * 1024;

const MAX_RESPONSE_HEADERS: usize = 16;
const MAX_REQUEST_HEADERS: usize = 16;
const MAX_HEADER_NAME: usize = 32;
const MAX_HEADER_VALUE: usize = 96;
const MAX_REASON_PHRASE: usize = 32;

/// Buffer used to transport HTTP requests to the Bluetooth service.
pub type HttpRequestBuffer = HeaplessVec<u8, MAX_HTTP_REQUEST>;
/// Buffer used to transport complete HTTP responses from the Bluetooth service.
pub type HttpResponseBuffer = Vec<u8>;

/// Heap-allocated HTTP header entry.
#[derive(Clone, Debug)]
pub struct HttpHeader {
    pub name: String<MAX_HEADER_NAME>,
    pub value: String<MAX_HEADER_VALUE>,
}

/// Fixed-capacity collection of HTTP response headers.
pub type HttpHeaderList = HeaplessVec<HttpHeader, MAX_RESPONSE_HEADERS>;

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
    InvalidJson,
    NoConnection,
    Disconnected,
    Internal,
    InvalidMethod,
    HeadersTooMany,
    HeaderNameTooLong,
    HeaderValueTooLong,
}

impl defmt::Format for HttpBridgeError {
    fn format(&self, f: defmt::Formatter) {
        let label = match self {
            Self::RequestTooLarge => "RequestTooLarge",
            Self::ResponseTooLarge => "ResponseTooLarge",
            Self::InvalidUtf8 => "InvalidUtf8",
            Self::InvalidResponse => "InvalidResponse",
            Self::InvalidJson => "InvalidJson",
            Self::NoConnection => "NoConnection",
            Self::Disconnected => "Disconnected",
            Self::Internal => "Internal",
            Self::InvalidMethod => "InvalidMethod",
            Self::HeadersTooMany => "HeadersTooMany",
            Self::HeaderNameTooLong => "HeaderNameTooLong",
            Self::HeaderValueTooLong => "HeaderValueTooLong",
        };
        defmt::write!(f, "HttpBridgeError::{}", label);
    }
}

/// Parsed HTTP response returned to callers.
pub struct HttpResponse {
    pub status: u16,
    pub reason: String<MAX_REASON_PHRASE>,
    pub headers: HttpHeaderList,
    pub body: Vec<u8>,
}

impl HttpResponse {
    /// Returns the response body as raw bytes.
    pub fn body(&self) -> &[u8] {
        self.body.as_slice()
    }

    /// Attempts to view the response body as UTF-8 text.
    pub fn body_text(&self) -> Result<&str, HttpBridgeError> {
        core::str::from_utf8(self.body.as_slice()).map_err(|_| HttpBridgeError::InvalidUtf8)
    }

    /// Retrieves a response header by (case-insensitive) name.
    pub fn header(&self, name: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|header| ascii_eq_ignore_case(header.name.as_str(), name))
            .map(|header| header.value.as_str())
    }

    /// Attempts to deserialize the response body as JSON.
    pub fn json<T>(&self) -> Result<T, HttpBridgeError>
    where
        T: DeserializeOwned,
    {
        json::from_slice::<T>(self.body.as_slice())
            .map(|(value, _)| value)
            .map_err(|_| HttpBridgeError::InvalidJson)
    }
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

/// Lightweight HTTP client for issuing requests over the BLE bridge.
#[derive(Clone, Copy, Debug, Default)]
pub struct HttpClient;

impl HttpClient {
    /// Construct a new client instance.
    pub const fn new() -> Self {
        Self
    }

    /// Create a request builder for the provided method and URL.
    pub fn request<'a>(&'a self, method: &'a str, url: &'a str) -> RequestBuilder<'a> {
        RequestBuilder {
            client: self,
            method,
            url,
            headers: HeaplessVec::new(),
            body: None,
        }
    }

    /// Create a `GET` request builder for the specified URL.
    pub fn get<'a>(&'a self, url: &'a str) -> RequestBuilder<'a> {
        self.request("GET", url)
    }

    /// Create a `POST` request builder for the specified URL.
    pub fn post<'a>(&'a self, url: &'a str) -> RequestBuilder<'a> {
        self.request("POST", url)
    }

    async fn execute(
        &self,
        method: &str,
        url: &str,
        headers: &[(&str, &str)],
        body: Option<&[u8]>,
    ) -> Result<HttpResponse, HttpBridgeError> {
        perform_request(method, url, headers, body).await
    }
}

/// Builder providing a reqwest-like interface for constructing BLE HTTP requests.
pub struct RequestBuilder<'a> {
    client: &'a HttpClient,
    method: &'a str,
    url: &'a str,
    headers: HeaplessVec<(&'a str, &'a str), MAX_REQUEST_HEADERS>,
    body: Option<&'a [u8]>,
}

impl<'a> RequestBuilder<'a> {
    /// Append a header to the outbound request.
    pub fn header(mut self, name: &'a str, value: &'a str) -> Result<Self, HttpBridgeError> {
        self.headers
            .push((name, value))
            .map_err(|_| HttpBridgeError::HeadersTooMany)?;
        Ok(self)
    }

    /// Append multiple headers to the outbound request.
    pub fn headers(mut self, headers: &'a [(&'a str, &'a str)]) -> Result<Self, HttpBridgeError> {
        for header in headers {
            self.headers
                .push(*header)
                .map_err(|_| HttpBridgeError::HeadersTooMany)?;
        }
        Ok(self)
    }

    /// Set the request body payload.
    pub fn body(mut self, body: &'a [u8]) -> Self {
        self.body = Some(body);
        self
    }

    /// Finalise the request and await the response.
    pub async fn send(self) -> Result<HttpResponse, HttpBridgeError> {
        self.client
            .execute(self.method, self.url, self.headers.as_slice(), self.body)
            .await
    }
}

async fn perform_request(
    method: &str,
    path: &str,
    headers: &[(&str, &str)],
    body: Option<&[u8]>,
) -> Result<HttpResponse, HttpBridgeError> {
    validate_method(method)?;

    let mut request = HttpRequestBuffer::new();
    build_request(&mut request, method, path, headers, body)?;

    let raw = send_raw_request(request.as_slice()).await?;
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
    let mut headers = [EMPTY_HEADER; MAX_RESPONSE_HEADERS];
    let mut response = HttpParseResponse::new(&mut headers);

    let header_len = match response.parse(raw) {
        Ok(Status::Complete(len)) => len,
        Ok(Status::Partial) => return Err(HttpBridgeError::InvalidResponse),
        Err(_) => return Err(HttpBridgeError::InvalidResponse),
    };

    let status = response
        .code
        .ok_or(HttpBridgeError::InvalidResponse)?
        .try_into()
        .map_err(|_| HttpBridgeError::InvalidResponse)?;

    let mut reason = String::<MAX_REASON_PHRASE>::new();
    if let Some(reason_str) = response.reason {
        reason
            .push_str(reason_str)
            .map_err(|_| HttpBridgeError::HeaderValueTooLong)?;
    }

    let mut header_list = HttpHeaderList::new();
    for header in response.headers.iter() {
        let mut name = String::<MAX_HEADER_NAME>::new();
        name.push_str(header.name)
            .map_err(|_| HttpBridgeError::HeaderNameTooLong)?;

        let value_str =
            core::str::from_utf8(header.value).map_err(|_| HttpBridgeError::InvalidUtf8)?;
        let mut value = String::<MAX_HEADER_VALUE>::new();
        value
            .push_str(value_str)
            .map_err(|_| HttpBridgeError::HeaderValueTooLong)?;

        header_list
            .push(HttpHeader { name, value })
            .map_err(|_| HttpBridgeError::HeadersTooMany)?;
    }

    let body_bytes = raw.get(header_len..).unwrap_or(&[]);

    if body_bytes.len() > MAX_HTTP_BODY {
        return Err(HttpBridgeError::ResponseTooLarge);
    }

    let mut body = Vec::with_capacity(body_bytes.len());
    body.extend_from_slice(body_bytes);
    Ok(HttpResponse {
        status,
        reason,
        headers: header_list,
        body,
    })
}

fn build_request(
    buffer: &mut HttpRequestBuffer,
    method: &str,
    path: &str,
    headers: &[(&str, &str)],
    body: Option<&[u8]>,
) -> Result<(), HttpBridgeError> {
    push_bytes(buffer, method.as_bytes())?;
    push_bytes(buffer, b" ")?;

    let absolute = is_absolute_url(path);
    if absolute || path.starts_with('/') {
        push_bytes(buffer, path.as_bytes())?;
    } else {
        push_bytes(buffer, b"/")?;
        push_bytes(buffer, path.as_bytes())?;
    }

    push_bytes(buffer, b" HTTP/1.1\r\n")?;

    let mut host_present = false;
    let mut content_length_present = false;
    let mut connection_present = false;

    for (name, value) in headers.iter() {
        if ascii_eq_ignore_case(name, "Host") {
            host_present = true;
        }
        if ascii_eq_ignore_case(name, "Content-Length") {
            content_length_present = true;
        }
        if ascii_eq_ignore_case(name, "Connection") {
            connection_present = true;
        }
        push_header(buffer, name, value)?;
    }

    if !host_present {
        if absolute {
            if let Some(host) = extract_host(path) {
                push_header(buffer, "Host", host)?;
            }
        } else {
            push_header(buffer, "Host", DEFAULT_HOST)?;
        }
    }

    if !connection_present {
        push_header(buffer, "Connection", "close")?;
    }

    if let Some(body_bytes) = body {
        if !content_length_present {
            let mut len_buf = String::<12>::new();
            write!(&mut len_buf, "{}", body_bytes.len()).map_err(|_| HttpBridgeError::Internal)?;
            push_header(buffer, "Content-Length", len_buf.as_str())?;
        }
    }

    push_bytes(buffer, b"\r\n")?;

    if let Some(body_bytes) = body {
        push_bytes(buffer, body_bytes)?;
    }

    Ok(())
}

fn push_header(
    buffer: &mut HttpRequestBuffer,
    name: &str,
    value: &str,
) -> Result<(), HttpBridgeError> {
    validate_header_name(name)?;
    push_bytes(buffer, name.as_bytes())?;
    push_bytes(buffer, b": ")?;
    push_bytes(buffer, value.as_bytes())?;
    push_bytes(buffer, b"\r\n")?;
    Ok(())
}

fn is_absolute_url(path: &str) -> bool {
    path.starts_with("http://") || path.starts_with("https://")
}

fn extract_host(url: &str) -> Option<&str> {
    let (_, rest) = url.split_once("://")?;
    let host_port = rest.split('/').next().unwrap_or(rest);
    if host_port.is_empty() {
        None
    } else {
        Some(host_port)
    }
}

fn validate_method(method: &str) -> Result<(), HttpBridgeError> {
    if method.is_empty() || method.len() > 16 {
        return Err(HttpBridgeError::InvalidMethod);
    }
    if !method.bytes().all(|b| matches!(b, b'A'..=b'Z' | b'-')) {
        return Err(HttpBridgeError::InvalidMethod);
    }
    Ok(())
}

fn validate_header_name(name: &str) -> Result<(), HttpBridgeError> {
    if name.is_empty() || name.len() > MAX_HEADER_NAME {
        return Err(HttpBridgeError::HeaderNameTooLong);
    }
    if !name
        .bytes()
        .all(|b| matches!(b, b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-'))
    {
        return Err(HttpBridgeError::HeaderNameTooLong);
    }
    Ok(())
}

fn push_bytes(buffer: &mut HttpRequestBuffer, data: &[u8]) -> Result<(), HttpBridgeError> {
    buffer
        .extend_from_slice(data)
        .map_err(|_| HttpBridgeError::RequestTooLarge)
}

fn ascii_eq_ignore_case(lhs: &str, rhs: &str) -> bool {
    if lhs.len() != rhs.len() {
        return false;
    }
    lhs.bytes()
        .zip(rhs.bytes())
        .all(|(a, b)| ascii_lower(a) == ascii_lower(b))
}

fn ascii_lower(byte: u8) -> u8 {
    if (b'A'..=b'Z').contains(&byte) {
        byte + 32
    } else {
        byte
    }
}
