// HTTP Response type

use crate::libs::hps::types::{HttpResponse as HpsResponse, HttpStatusCode};
use crate::libs::http::error::{Error, Result};
use defmt::Format;
use alloc::string::String;
use alloc::vec::Vec;

/// HTTP Response
/// 
/// Provides methods to access response data similar to reqwest::Response
pub struct Response {
    status_code: u16,
    headers: String, // Heap-allocated response headers
    body: Vec<u8>,   // Heap-allocated response body
}

// Manual Format implementation since alloc::String doesn't implement defmt::Format
impl defmt::Format for Response {
    fn format(&self, f: defmt::Formatter) {
        defmt::write!(f, "Response {{ status: {}, headers_len: {}, body_len: {} }}", 
            self.status_code, self.headers.len(), self.body.len())
    }
}

impl Response {
    /// Create a new Response from HPS response
    pub(crate) fn from_hps(hps_response: HpsResponse) -> Self {
        Self {
            status_code: hps_response.status_code,
            headers: String::from(hps_response.headers.as_str()),
            body: hps_response.body.to_vec(),
        }
    }
    
    /// Get the HTTP status code
    pub fn status(&self) -> u16 {
        self.status_code
    }
    
    /// Check if the status code indicates success (2xx)
    pub fn is_success(&self) -> bool {
        self.status_code >= 200 && self.status_code < 300
    }
    
    /// Check if the status code indicates a client error (4xx)
    pub fn is_client_error(&self) -> bool {
        self.status_code >= 400 && self.status_code < 500
    }
    
    /// Check if the status code indicates a server error (5xx)
    pub fn is_server_error(&self) -> bool {
        self.status_code >= 500 && self.status_code < 600
    }
    
    /// Get the response body as bytes
    pub fn bytes(&self) -> &[u8] {
        &self.body
    }
    
    /// Get the response body as a UTF-8 string
    pub fn text(&self) -> Result<&str> {
        core::str::from_utf8(&self.body).map_err(|_| Error::ResponseError)
    }
    
    /// Get the content length
    pub fn content_length(&self) -> usize {
        self.body.len()
    }
    
    /// Get response headers as a string
    pub fn headers(&self) -> &str {
        self.headers.as_str()
    }
    
    /// Get a specific header value by name (case-insensitive)
    pub fn header(&self, name: &str) -> Option<&str> {
        let name_lower = name.to_lowercase();
        for line in self.headers.lines() {
            if let Some(colon_pos) = line.find(':') {
                let header_name = line[..colon_pos].trim().to_lowercase();
                if header_name == name_lower {
                    return Some(line[colon_pos + 1..].trim());
                }
            }
        }
        None
    }
    
    /// Parse response body as JSON
    /// Note: This returns the raw body for now - JSON parsing is done by the app
    #[cfg(feature = "json")]
    pub fn json<T>(&self) -> Result<T> 
    where
        T: serde::de::DeserializeOwned,
    {
        serde_json::from_slice(&self.body).map_err(|_| Error::JsonError)
    }
}
