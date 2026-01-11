// HTTP Request builder

use crate::libs::hps::types::{
    HttpMethod, HttpRequest as HpsRequest, MAX_BODY_SIZE, MAX_HEADERS_SIZE, MAX_URI_SIZE,
};
use crate::libs::http::client::Client;
use crate::libs::http::error::{Error, Result};
use crate::libs::http::response::Response;
use defmt::Format;
use heapless::{String, Vec};

/// HTTP Request builder
///
/// Build and send HTTP requests similar to reqwest::RequestBuilder
#[derive(Format)]
pub struct RequestBuilder {
    client: Client,
    method: HttpMethod,
    url: String<MAX_URI_SIZE>,
    headers: String<MAX_HEADERS_SIZE>,
    body: Vec<u8, MAX_BODY_SIZE>,
}

impl RequestBuilder {
    /// Create a new request builder
    pub(crate) fn new(client: Client, method: HttpMethod, url: &str) -> Result<Self> {
        // Parse URL and ensure it fits
        let url_string = String::try_from(url).map_err(|_| Error::InvalidUrl)?;

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
        if !self.headers.is_empty() {
            let _ = self.headers.push_str("\r\n");
        }
        let _ = self.headers.push_str(key);
        let _ = self.headers.push_str(": ");
        let _ = self.headers.push_str(value);
        self
    }

    /// Set the request body from bytes
    pub fn body(mut self, body: &[u8]) -> Result<Self> {
        self.body = Vec::from_slice(body).map_err(|_| Error::RequestTooLarge)?;
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
        self.body = Vec::from_slice(&json_bytes).map_err(|_| Error::RequestTooLarge)?;

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
