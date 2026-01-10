// HTTP Client implementation

use crate::libs::hps::types::{HttpMethod, HttpRequest as HpsRequest};
use crate::libs::http::error::{Error, Result};
use crate::libs::http::request::RequestBuilder;
use crate::libs::http::response::Response;
use crate::system::services::http_service::{http_request_sender, http_response_receiver};
use defmt::info;

/// HTTP Client
/// 
/// Provides a reqwest-like interface for making HTTP requests over BLE HPS
/// 
/// # Example
/// ```rust
/// let client = http::Client::new();
/// let response = client.get("https://api.example.com/data").send().await?;
/// println!("Status: {}", response.status());
/// ```
#[derive(Clone, Copy, defmt::Format)]
pub struct Client {
    // Stateless client - all state is in the http_service
}

impl Client {
    /// Create a new HTTP client
    pub fn new() -> Self {
        Self {}
    }
    
    /// Make a GET request to a URL
    pub fn get(&self, url: &str) -> RequestBuilder {
        self.request(HttpMethod::Get, url)
    }
    
    /// Make a GET request with HTTPS (secure)
    pub fn get_secure(&self, url: &str) -> RequestBuilder {
        self.request(HttpMethod::GetSecure, url)
    }
    
    /// Make a POST request to a URL
    pub fn post(&self, url: &str) -> RequestBuilder {
        self.request(HttpMethod::Post, url)
    }
    
    /// Make a POST request with HTTPS (secure)
    pub fn post_secure(&self, url: &str) -> RequestBuilder {
        self.request(HttpMethod::PostSecure, url)
    }
    
    /// Make a PUT request to a URL
    pub fn put(&self, url: &str) -> RequestBuilder {
        self.request(HttpMethod::Put, url)
    }
    
    /// Make a PUT request with HTTPS (secure)
    pub fn put_secure(&self, url: &str) -> RequestBuilder {
        self.request(HttpMethod::PutSecure, url)
    }
    
    /// Make a DELETE request to a URL
    pub fn delete(&self, url: &str) -> RequestBuilder {
        self.request(HttpMethod::Delete, url)
    }
    
    /// Make a DELETE request with HTTPS (secure)
    pub fn delete_secure(&self, url: &str) -> RequestBuilder {
        self.request(HttpMethod::DeleteSecure, url)
    }
    
    /// Make a HEAD request to a URL
    pub fn head(&self, url: &str) -> RequestBuilder {
        self.request(HttpMethod::Head, url)
    }
    
    /// Make a HEAD request with HTTPS (secure)
    pub fn head_secure(&self, url: &str) -> RequestBuilder {
        self.request(HttpMethod::HeadSecure, url)
    }
    
    /// Make a request with a specific method
    fn request(&self, method: HttpMethod, url: &str) -> RequestBuilder {
        RequestBuilder::new(*self, method, url).unwrap_or_else(|_| {
            // If URL is invalid, create a dummy builder that will error on send()
            RequestBuilder::new(*self, method, "").unwrap()
        })
    }
    
    /// Send an HTTP request (internal method used by RequestBuilder)
    pub(crate) async fn send_request(&self, request: HpsRequest<'_>) -> Result<Response> {
        info!("HTTP Client: Sending {} request to {}", request.method, request.uri);
        
        // Convert to owned types for channel
        let method = request.method;
        let uri = heapless::String::try_from(request.uri).map_err(|_| Error::InvalidUrl)?;
        let headers = heapless::String::try_from(request.headers).map_err(|_| Error::InvalidUrl)?;
        let body = heapless::Vec::from_slice(request.body).map_err(|_| Error::RequestTooLarge)?;
        
        // Create service request
        let service_request = crate::system::services::http_service::HttpServiceRequest {
            method,
            uri,
            headers,
            body,
        };
        
        // Send to service
        let tx = http_request_sender();
        tx.send(service_request).await;
        
        // Wait for response
        let rx = http_response_receiver();
        let result = rx.receive().await;
        
        // Convert result
        match result {
            Ok(hps_response) => Ok(Response::from_hps(hps_response)),
            Err(hps_error) => Err(Error::from(hps_error)),
        }
    }
}

impl Default for Client {
    fn default() -> Self {
        Self::new()
    }
}
