use alloc::string::String;
use alloc::vec::Vec;
use defmt::{write, Format, Formatter};

/// HPS Service and Characteristic UUIDs (Bluetooth SIG Assigned Numbers)
pub struct HpsUuids;

impl HpsUuids {
    /// HTTP Proxy Service UUID: 0x1823
    pub const SERVICE: u16 = 0x1823;

    /// URI Characteristic UUID: 0x2AB6
    pub const URI: u16 = 0x2AB6;

    /// HTTP Headers Characteristic UUID: 0x2AB7
    pub const HTTP_HEADERS: u16 = 0x2AB7;

    /// HTTP Status Code Characteristic UUID: 0x2AB8
    pub const HTTP_STATUS_CODE: u16 = 0x2AB8;

    /// HTTP Entity Body Characteristic UUID: 0x2AB9
    pub const HTTP_ENTITY_BODY: u16 = 0x2AB9;

    /// HTTP Control Point Characteristic UUID: 0x2ABA
    pub const HTTP_CONTROL_POINT: u16 = 0x2ABA;

    /// HTTPS Security Characteristic UUID: 0x2ABB
    pub const HTTPS_SECURITY: u16 = 0x2ABB;
}

/// HTTP Methods supported by HPS
#[derive(Debug, Clone, Copy, Format, PartialEq, Eq)]
#[repr(u8)]
pub enum HttpMethod {
    Get = 0x01,
    Head = 0x02,
    Post = 0x03,
    Put = 0x04,
    Delete = 0x05,
    GetSecure = 0x06,
    HeadSecure = 0x07,
    PostSecure = 0x08,
    PutSecure = 0x09,
    DeleteSecure = 0x0A,
    Cancel = 0x0B,
}

impl HttpMethod {
    pub fn is_secure(&self) -> bool {
        matches!(
            self,
            HttpMethod::GetSecure
                | HttpMethod::HeadSecure
                | HttpMethod::PostSecure
                | HttpMethod::PutSecure
                | HttpMethod::DeleteSecure
        )
    }
}

/// Data Status bit field from HTTP Status Code characteristic
#[derive(Debug, Clone, Copy, Format, Default)]
pub struct DataStatus {
    /// Headers were received
    pub headers_received: bool,

    /// Headers were truncated (exceeded 512 octets)
    pub headers_truncated: bool,

    /// Body was received
    pub body_received: bool,

    /// Body was truncated (exceeded 512 octets)
    pub body_truncated: bool,
}

impl DataStatus {
    pub fn from_byte(byte: u8) -> Self {
        Self {
            headers_received: (byte & 0x01) != 0,
            headers_truncated: (byte & 0x02) != 0,
            body_received: (byte & 0x04) != 0,
            body_truncated: (byte & 0x08) != 0,
        }
    }

    pub fn to_byte(&self) -> u8 {
        let mut byte = 0u8;
        if self.headers_received {
            byte |= 0x01;
        }
        if self.headers_truncated {
            byte |= 0x02;
        }
        if self.body_received {
            byte |= 0x04;
        }
        if self.body_truncated {
            byte |= 0x08;
        }
        byte
    }
}

/// HTTP Status Code (status code + data status)
#[derive(Debug, Clone, Copy, Format)]
pub struct HttpStatusCode {
    /// HTTP status code (e.g., 200, 404, 500)
    pub status_code: u16,

    /// Data status bit field
    pub data_status: DataStatus,
}

impl HttpStatusCode {
    /// Parse from 3-octet notification (uint16 status + uint8 data status)
    pub fn from_bytes(bytes: &[u8]) -> Option<Self> {
        if bytes.len() < 3 {
            return None;
        }

        // Little endian uint16
        let status_code = u16::from_le_bytes([bytes[0], bytes[1]]);
        let data_status = DataStatus::from_byte(bytes[2]);

        Some(Self {
            status_code,
            data_status,
        })
    }

    pub fn is_success(&self) -> bool {
        self.status_code >= 200 && self.status_code < 300
    }
}

/// Maximum sizes per HPS v1.0 specification
/// Spec allows up to 512 bytes per characteristic, but we use larger buffers
/// Total budget: 10KB across all characteristics
pub const MAX_URI_SIZE: usize = 1024; // 1KB for URLs
pub const MAX_HEADERS_SIZE: usize = 4096; // 4KB for request + response headers
pub const MAX_BODY_SIZE: usize = 5120; // 5KB for request + response body

/// HTTP Request to be sent via HPS
pub struct HttpRequest<'a> {
    pub method: HttpMethod,
    pub uri: &'a str,
    pub headers: &'a str,
    pub body: &'a [u8],
}

impl<'a> HttpRequest<'a> {
    pub fn get(uri: &'a str) -> Self {
        Self {
            method: HttpMethod::Get,
            uri,
            headers: "",
            body: &[],
        }
    }

    pub fn post(uri: &'a str, body: &'a [u8]) -> Self {
        Self {
            method: HttpMethod::Post,
            uri,
            headers: "",
            body,
        }
    }

    pub fn with_headers(mut self, headers: &'a str) -> Self {
        self.headers = headers;
        self
    }
}

/// HTTP Response received via HPS
#[derive(Debug, Clone)]
pub struct HttpResponse {
    pub status_code: u16,
    pub data_status: DataStatus,
    pub headers: String,
    pub body: Vec<u8>,
}

impl Default for HttpResponse {
    fn default() -> Self {
        Self {
            status_code: 0,
            data_status: DataStatus::default(),
            headers: String::new(),
            body: Vec::new(),
        }
    }
}

impl HttpResponse {
    pub fn is_success(&self) -> bool {
        self.status_code >= 200 && self.status_code < 300
    }
}

impl Format for HttpResponse {
    fn format(&self, fmt: Formatter) {
        write!(
            fmt,
            "HttpResponse {{ status: {}, data_status: {:?}, headers_len: {}, body_len: {} }}",
            self.status_code,
            self.data_status,
            self.headers.len(),
            self.body.len()
        );
    }
}

/// HPS Characteristic handles (discovered during service discovery)
#[derive(Debug, Clone, Copy, Default)]
pub struct HpsCharacteristics {
    pub uri: Option<u16>,
    pub headers: Option<u16>,
    pub entity_body: Option<u16>,
    pub control_point: Option<u16>,
    pub status_code: Option<u16>,
    pub status_code_cccd: Option<u16>,
    pub https_security: Option<u16>,
}

impl HpsCharacteristics {
    pub fn is_complete(&self) -> bool {
        self.uri.is_some()
            && self.headers.is_some()
            && self.entity_body.is_some()
            && self.control_point.is_some()
            && self.status_code.is_some()
            && self.status_code_cccd.is_some()
            && self.https_security.is_some()
    }
}
