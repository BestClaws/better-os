// HTTP Service
// Provides HTTP abstraction layer over HPS (BLE proxy)
// Apps use libs/http for clean API, this service handles the bridge to HPS

use defmt::info;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::channel::{Channel, Receiver, Sender};
use alloc::boxed::Box;
use alloc::string::String;
use alloc::vec::Vec;

use crate::libs::hps::error::HpsError;
use crate::libs::hps::types::{HttpMethod, HttpResponse, MAX_BODY_SIZE, MAX_HEADERS_SIZE, MAX_URI_SIZE};
use crate::system::services::hps_service::HpsRequest;

/// HTTP service response
pub type HttpServiceResponse = Result<HttpResponse, HpsError>;

/// Channel for HTTP requests (from apps via http::Client) - passes HpsRequest directly
static HTTP_REQUEST_CHANNEL: Channel<CriticalSectionRawMutex, HpsRequest, 2> =
    Channel::new();

/// Channel for HTTP responses (to apps via http::Client)
static HTTP_RESPONSE_CHANNEL: Channel<CriticalSectionRawMutex, HttpServiceResponse, 2> =
    Channel::new();

/// Get sender for HTTP requests (used by libs/http/client.rs)
pub fn http_request_sender() -> Sender<'static, CriticalSectionRawMutex, HpsRequest, 2> {
    HTTP_REQUEST_CHANNEL.sender()
}

/// Get receiver for HTTP responses (used by libs/http/client.rs)
pub fn http_response_receiver(
) -> Receiver<'static, CriticalSectionRawMutex, HttpServiceResponse, 2> {
    HTTP_RESPONSE_CHANNEL.receiver()
}

/// HTTP service task
/// Bridges HTTP client API to HPS service
#[embassy_executor::task]
pub(crate) async fn http_service() {
    info!("HTTP service starting");

    let request_rx = HTTP_REQUEST_CHANNEL.receiver();
    let response_tx = HTTP_RESPONSE_CHANNEL.sender();

    // Get HPS service channels
    let hps_request_tx = crate::system::services::hps_service::hps_request_sender();
    let hps_response_rx = crate::system::services::hps_service::hps_response_receiver();

    info!("HTTP service ready, waiting for requests");

    loop {
        // Wait for HTTP request from client (already HpsRequest)
        let hps_request = request_rx.receive().await;
        
        info!(
            "HTTP service: Forwarding {} request",
            hps_request.method
        );

        // Forward directly to HPS service
        hps_request_tx.send(hps_request).await;

        // Wait for HPS response
        let hps_response = hps_response_rx.receive().await;

        // Forward back to HTTP client
        response_tx.send(hps_response).await;
    }
}
