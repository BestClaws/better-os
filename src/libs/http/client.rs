// HTTP Client implementation

use crate::libs::hps::types::{HttpMethod, HttpRequest as HpsRequest, MAX_URI_SIZE, MAX_HEADERS_SIZE, MAX_BODY_SIZE};
use crate::libs::http::error::{Error, Result};
use crate::libs::http::request::RequestBuilder;
use crate::libs::http::response::Response;
use crate::system::services::http_service::{http_request_sender, http_response_receiver};
use defmt::{info, debug};

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
        debug!("HTTP Client: Sending {} request to {}", request.method, request.uri);
        
        // Validate sizes
        if request.uri.len() > MAX_URI_SIZE {
            return Err(Error::InvalidUrl);
        }
        if request.headers.len() > MAX_HEADERS_SIZE {
            return Err(Error::InvalidUrl);
        }
        if request.body.len() > MAX_BODY_SIZE {
            return Err(Error::RequestTooLarge);
        }
        
        // Create HpsRequest directly - no boxing needed!
        // String and Vec are already heap-allocated internally
        let hps_request = crate::system::services::hps_service::HpsRequest {
            method: request.method,
            uri: alloc::string::String::from(request.uri),
            headers: alloc::string::String::from(request.headers),
            body: request.body.to_vec(),
        };
        
        debug!("HTTP Client: Sending request through channel...");
        // Send directly to service
        let tx = http_request_sender();
        tx.send(hps_request).await;
        
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
