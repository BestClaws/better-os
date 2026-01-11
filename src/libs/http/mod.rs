// HTTP client library with reqwest-like API
// Abstracts BLE HPS operations behind a clean HTTP interface

mod client;
mod error;
mod request;
mod response;

pub use client::Client;
pub use error::{Error, Result};
pub use request::RequestBuilder;
pub use response::Response;

use crate::libs::hps::types::HttpMethod;

/// Convenience function to make a GET request
///
/// # Example
/// ```rust
/// let response = http::get("https://api.example.com/data").await?;
/// let body = response.text().await?;
/// ```
pub async fn get(url: &str) -> Result<Response> {
    Client::new().get(url).send().await
}

/// Convenience function to make a POST request
pub async fn post(url: &str) -> RequestBuilder {
    Client::new().post(url)
}

/// Convenience function to make a PUT request
pub async fn put(url: &str) -> RequestBuilder {
    Client::new().put(url)
}

/// Convenience function to make a DELETE request
pub async fn delete(url: &str) -> RequestBuilder {
    Client::new().delete(url)
}

/// Convenience function to make a HEAD request
pub async fn head(url: &str) -> RequestBuilder {
    Client::new().head(url)
}
