// HTTP Response type

use crate::libs::hps::types::{HttpResponse as HpsResponse, HttpStatusCode};
use crate::libs::http::error::{Error, Result};
use defmt::Format;
use heapless::Vec;

/// HTTP Response
/// 
/// Provides methods to access response data similar to reqwest::Response
#[derive(Format)]
pub struct Response {
    status_code: u16,
    body: Vec<u8, 512>, // HPS max body size
}

impl Response {
    /// Create a new Response from HPS response
    pub(crate) fn from_hps(hps_response: HpsResponse) -> Self {
        Self {
            status_code: hps_response.status_code,
            body: hps_response.body,
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
