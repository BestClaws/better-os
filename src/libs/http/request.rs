// HTTP Request builder

use crate::libs::hps::types::{
    HttpMethod, HttpRequest as HpsRequest, MAX_BODY_SIZE, MAX_HEADERS_SIZE, MAX_URI_SIZE,
};
use crate::libs::http::client::Client;
use crate::libs::http::error::{Error, Result};
use crate::libs::http::response::Response;
use alloc::string::String;
use alloc::vec::Vec;
use defmt::{write, Format, Formatter};

/// HTTP Request builder
///
/// Build and send HTTP requests similar to reqwest::RequestBuilder
pub struct RequestBuilder {
    client: Client,
    method: HttpMethod,
    url: String,
    headers: String,
    body: Vec<u8>,
}

impl Format for RequestBuilder {
    fn format(&self, fmt: Formatter) {
        // Log minimal details to avoid formatting large buffers
        write!(
            fmt,
            "RequestBuilder {{ method: {}, url: {=str}, headers_len: {}, body_len: {} }}",
            self.method,
            self.url.as_str(),
            self.headers.len(),
            self.body.len()
        );
    }
}

impl RequestBuilder {
    /// Create a new request builder
    pub(crate) fn new(client: Client, method: HttpMethod, url: &str) -> Result<Self> {
        // Parse URL and ensure it fits
        if url.len() > MAX_URI_SIZE {
            return Err(Error::InvalidUrl);
        }
        let url_string = String::from(url);

        Ok(Self {
            client,
            method,
            url: url_string,
            headers: String::new(),
            body: Vec::new(),
        })
    }

    /// Add a header to the request
    pub fn header(mut self, key: &str, value: &str) -> Self {
        // Append "Key: Value\r\n" to headers string
        let needs_newline = !self.headers.is_empty();
        let additional_len = key.len() + value.len() + 2 + if needs_newline { 2 } else { 0 };

        if self.headers.len() + additional_len <= MAX_HEADERS_SIZE {
            if needs_newline {
                self.headers.push_str("\r\n");
            }
            self.headers.push_str(key);
            self.headers.push_str(": ");
            self.headers.push_str(value);
        }
        self
    }

    /// Set the request body from bytes
    pub fn body(mut self, body: &[u8]) -> Result<Self> {
        if body.len() > MAX_BODY_SIZE {
            return Err(Error::RequestTooLarge);
        }
        self.body.clear();
        self.body.extend_from_slice(body);
        Ok(self)
    }

    /// Set the request body from a string
    pub fn body_str(self, body: &str) -> Result<Self> {
        self.body(body.as_bytes())
    }

    /// Set the request body as JSON
    #[cfg(feature = "json")]
    pub fn json<T: serde::Serialize>(mut self, json: &T) -> Result<Self> {
        // Serialize to heapless Vec
        let json_bytes = serde_json::to_vec(json).map_err(|_| Error::JsonError)?;
        if json_bytes.len() > MAX_BODY_SIZE {
            return Err(Error::RequestTooLarge);
        }
        self.body = json_bytes;

        // Add Content-Type header
        self = self.header("Content-Type", "application/json");
        Ok(self)
    }

    /// Send the request and wait for response
    pub async fn send(self) -> Result<Response> {
        // Build HPS request
        let hps_request = HpsRequest {
            method: self.method,
            uri: self.url.as_str(),
            headers: self.headers.as_str(),
            body: &self.body,
        };

        // Send via client
        self.client.send_request(hps_request).await
    }

    /// Build the request without sending
    pub fn build(self) -> Result<HpsRequest<'static>> {
        // This would require 'static lifetime management
        // For now, we only support send()
        Err(Error::InvalidUrl)
    }
}
