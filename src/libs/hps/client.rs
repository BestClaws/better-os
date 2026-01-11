use crate::libs::hps::error::HpsError;
use crate::libs::hps::types::{
    HttpMethod, HttpRequest, HttpResponse, MAX_BODY_SIZE, MAX_HEADERS_SIZE, MAX_URI_SIZE,
};
use crate::system::services::hps_service::{hps_request_sender, hps_response_receiver};
use defmt::{debug, info};

/// HPS Client for making HTTP requests through a BLE HPS Server
/// 
/// This is a lightweight facade that communicates with the hps_service task
/// via channels. The actual BLE/GATT operations are handled by the service.
pub struct HpsClient {
    // Channel-based API - no direct BLE access
}

impl HpsClient {
    pub fn new() -> Self {
        debug!("HPS Client created (channel-based)");
        Self {}
    }
    
    /// Check if HPS service is available
    /// TODO: Add service status query via channel
    pub fn is_ready(&self) -> bool {
        // For now, always return true - service will handle errors
        true
    }
    
    /// Send an HTTP request and wait for response
    /// This sends the request to hps_service task and waits for response
    pub async fn send_request(&mut self, request: HttpRequest<'_>) -> Result<HttpResponse, HpsError> {
        debug!("HPS: Sending {} request to {}", request.method, request.uri);
        
        // Validate sizes
        if request.uri.len() > MAX_URI_SIZE {
            return Err(HpsError::BufferTooSmall);
        }
        if request.headers.len() > MAX_HEADERS_SIZE {
            return Err(HpsError::BufferTooSmall);
        }
        if request.body.len() > MAX_BODY_SIZE {
            return Err(HpsError::BufferTooSmall);
        }
        
        // Create HpsRequest directly - no boxing needed
        let hps_request = crate::system::services::hps_service::HpsRequest {
            method: request.method,
            uri: alloc::string::String::from(request.uri),
            headers: alloc::string::String::from(request.headers),
            body: request.body.to_vec(),
        };
        
        // Send request to service task
        let request_tx = hps_request_sender();
        request_tx.send(hps_request).await;
        
        // Wait for response from service task
        let response_rx = hps_response_receiver();
        let result = response_rx.receive().await;
        
        debug!("HPS: Received response from service");
        result
    }
    
    /// Cancel currently executing HTTP request
    pub async fn cancel_request(&mut self) -> Result<(), HpsError> {
        debug!("HPS: Canceling request");
        
        // Send cancel request (using HttpMethod::Cancel)
        let cancel_request = crate::system::services::hps_service::HpsRequest {
            method: HttpMethod::Cancel,
            uri: alloc::string::String::new(),
            headers: alloc::string::String::new(),
            body: alloc::vec::Vec::new(),
        };
        
        let request_tx = hps_request_sender();
        request_tx.send(cancel_request).await;
        
        Ok(())
    }
}

impl Default for HpsClient {
    fn default() -> Self {
        Self::new()
    }
}
