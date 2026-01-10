// HPS (HTTP Proxy Service) Client Service
// Manages BLE connection to HPS server and handles HTTP requests via GATT

use defmt::{info, warn, Debug2Format};
use embassy_futures::join::join;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::channel::{Channel, Receiver, Sender};
use embassy_sync::mutex::Mutex;
use embassy_time::{Duration, Timer, with_timeout};
use trouble_host::prelude::*;
use bt_hci::param::LeAdvReport;
use heapless::Vec;

use crate::libs::hps::error::HpsError;
use crate::libs::hps::types::{
    DataStatus, HpsCharacteristics, HpsUuids, HttpMethod, HttpRequest, HttpResponse, HttpStatusCode,
    MAX_BODY_SIZE, MAX_HEADERS_SIZE, MAX_URI_SIZE,
};
use crate::system::hal::radio::AsyncRadio;
use alloc::boxed::Box;
use core::cell::RefCell;

/// HPS request message
#[derive(Debug)]
pub struct HpsRequest {
    pub method: HttpMethod,
    pub uri: heapless::String<MAX_URI_SIZE>,
    pub headers: heapless::String<MAX_HEADERS_SIZE>,
    pub body: heapless::Vec<u8, MAX_BODY_SIZE>,
}

/// HPS response message
pub type HpsResponse = Result<HttpResponse, HpsError>;

/// Channel for HPS requests
static HPS_REQUEST_CHANNEL: Channel<CriticalSectionRawMutex, HpsRequest, 2> = Channel::new();

/// Channel for HPS responses
static HPS_RESPONSE_CHANNEL: Channel<CriticalSectionRawMutex, HpsResponse, 2> = Channel::new();

/// Channel for HPS ready signal - sent when connection is established
static HPS_READY_CHANNEL: Channel<CriticalSectionRawMutex, bool, 1> = Channel::new();

/// Get sender for HPS requests (for apps to use)
pub fn hps_request_sender() -> Sender<'static, CriticalSectionRawMutex, HpsRequest, 2> {
    HPS_REQUEST_CHANNEL.sender()
}

/// Get receiver for HPS responses (for apps to use)
pub fn hps_response_receiver() -> Receiver<'static, CriticalSectionRawMutex, HpsResponse, 2> {
    HPS_RESPONSE_CHANNEL.receiver()
}

/// Wait for HPS service to be ready
/// Returns immediately if already ready, otherwise blocks until ready signal is sent
pub async fn wait_for_hps_ready() {
    HPS_READY_CHANNEL.receiver().receive().await;
}

/// HPS service state
enum HpsServiceState {
    Disconnected,
    Connecting,
    Ready,
    Error,
}

/// HPS service task
/// This task manages the BLE connection to an HPS server and processes HTTP requests
#[embassy_executor::task]
pub(crate) async fn hps_service(
    radio: &'static Mutex<CriticalSectionRawMutex, Box<dyn AsyncRadio>>,
) {
    info!("HPS service starting");

    // Get BLE stack
    let mut _radio_guard = radio.lock().await;
    let stack = _radio_guard.get_stack().await;
    
    // Build host components - this borrows stack, doesn't consume it
    let Host {
        mut central,
        peripheral: _peripheral,
        mut runner,
        ..
    } = stack.build();

    let request_rx = HPS_REQUEST_CHANNEL.receiver();
    let response_tx = HPS_RESPONSE_CHANNEL.sender();
    
    // Create event handler for scanning
    let handler = HpsScanHandler::new();
    
    // Run BLE stack with event handler to receive scan results
    info!("HPS: Starting runner task...");
    let runner_task = async {
        info!("HPS: Runner task started, calling run_with_handler...");
        let result = runner.run_with_handler(&handler).await;
        warn!("HPS: Runner task ended with: {:?}", Debug2Format(&result));
        result
    };
    
    // Service task - note we need to keep stack reference for GATT client
    let service_task = async {
        let mut state = HpsServiceState::Disconnected;
        
        // Scan for HPS server first
        info!("HPS: Scanning for HPS server...");
        let mut scanner = Scanner::new(central);
        
        let mut scan_config = ScanConfig::default();
        scan_config.active = true;
        scan_config.phys = PhySet::M1;
        scan_config.interval = Duration::from_millis(100);
        scan_config.window = Duration::from_millis(100);
        scan_config.timeout = Duration::from_secs(10);
        scan_config.filter_accept_list = &[];  // Scan for any device
        
        info!("HPS: Starting 10-second scan to discover HPS server...");
        let hps_server_addr: Option<[u8; 6]> = match scanner.scan(&scan_config).await {
            Ok(_session) => {
                Timer::after(Duration::from_secs(10)).await;
                info!("HPS: Scan completed, checking if HPS server was found...");
                handler.take_found_device()
            }
            Err(e) => {
                warn!("HPS: Failed to start scan: {:?}", Debug2Format(&e));
                None
            }
        };
        
        let target_addr = if let Some(addr) = hps_server_addr {
            info!("HPS: Found HPS server at {=[u8]:02X}", addr);
            addr
        } else {
            warn!("HPS: HPS server not found during scan, using default address");
            // BLE addresses are LITTLE-ENDIAN: 14:13:33:36:4F:DA becomes [DA, 4F, 36, 33, 13, 14]
            [0xDA, 0x4F, 0x36, 0x33, 0x13, 0x14]
        };
        
        let target = Address {
            kind: AddrKind::PUBLIC,
            addr: BdAddr::new(target_addr),
        };
        
        // Get central back for connection
        let mut central = scanner.into_inner();
        
        loop {
            match state {
                HpsServiceState::Disconnected => {
                    info!("HPS: Disconnected, will attempt connection...");
                    Timer::after(Duration::from_secs(2)).await;
                    state = HpsServiceState::Connecting;
                }
                
                HpsServiceState::Connecting => {
                    info!("HPS: Connecting to HPS server at {=[u8]:02X}", target_addr);
                    info!("HPS: Target address kind: PUBLIC, trying connection...");
                    
                    let config = ConnectConfig {
                        connect_params: ConnectParams {
                            min_connection_interval: Duration::from_millis(50),
                            max_connection_interval: Duration::from_millis(100),
                            max_latency: 0,
                            min_event_length: Duration::from_millis(1),
                            max_event_length: Duration::from_millis(10),
                            supervision_timeout: Duration::from_secs(10),
                        },
                        scan_config: ScanConfig {
                            active: true,
                            filter_accept_list: &[(target.kind, &target.addr)],
                            phys: PhySet::M1,
                            interval: Duration::from_millis(100),
                            window: Duration::from_millis(100),  // 100% duty cycle
                            timeout: Duration::from_secs(20),
                        },
                    };
                    
                    info!("HPS: Calling central.connect() with 20s scan timeout...");
                    // Wrap in timeout to ensure we don't hang forever
                    let result = with_timeout(Duration::from_secs(25), central.connect(&config)).await;
                    
                    match result {
                        Ok(Ok(conn)) => {
                            info!("HPS: Connection established! Creating GATT client...");
                            
                            // Create GATT client - type inference will figure out the types
                            let gatt_client = match GattClient::<_, _, 10>::new(&stack, &conn).await {
                                Ok(client) => client,
                                Err(e) => {
                                    warn!("HPS: Failed to create GATT client: {:?}", Debug2Format(&e));
                                    drop(conn);
                                    Timer::after(Duration::from_secs(5)).await;
                                    state = HpsServiceState::Disconnected;
                                    continue;
                                }
                            };
                            
                            // Run GATT client task concurrently with service discovery and operations
                            info!("HPS: Starting GATT client tasks...");
                            let _ = join(
                                async {
                                    info!("HPS: GATT client task starting...");
                                    let result = gatt_client.task().await;
                                    warn!("HPS: GATT client task ended: {:?}", Debug2Format(&result));
                                    result
                                },
                                async {
                                    // Give GATT client task time to initialize
                                    info!("HPS: Waiting 2s for GATT client to initialize...");
                                    Timer::after(Duration::from_secs(2)).await;
                                    
                                    info!("HPS: Starting GATT service discovery for HPS (UUID 0x1823)...");
                                    let hps_uuid = Uuid::new_short(0x1823);
                                
                                    info!("HPS: Calling services_by_uuid() with 15s timeout...");
                                    let services_result = with_timeout(
                                        Duration::from_secs(15),
                                        gatt_client.services_by_uuid(&hps_uuid)
                                    ).await;
                                
                                let services = match services_result {
                                    Ok(Ok(svcs)) => {
                                        info!("HPS: Service discovery succeeded, found {} services", svcs.len());
                                        svcs
                                    }
                                    Ok(Err(e)) => {
                                        warn!("HPS: Service discovery failed: {:?}", Debug2Format(&e));
                                        drop(conn);
                                        return;
                                    }
                                    Err(_) => {
                                        warn!("HPS: Service discovery timeout after 10s");
                                        drop(conn);
                                        return;
                                    }
                                };
                                
                                if services.is_empty() {
                                    warn!("HPS: HPS service not found on server");
                                    drop(conn);
                                    return;
                                }
                                
                                let service = services.first().unwrap().clone();
                                info!("HPS: Service found, discovering characteristics...");
                                
                                // Discover HPS characteristics
                                let uri_uuid = Uuid::new_short(HpsUuids::URI);
                                let headers_uuid = Uuid::new_short(HpsUuids::HTTP_HEADERS);
                                let control_uuid = Uuid::new_short(HpsUuids::HTTP_CONTROL_POINT);
                                let status_uuid = Uuid::new_short(HpsUuids::HTTP_STATUS_CODE);
                                let body_uuid = Uuid::new_short(HpsUuids::HTTP_ENTITY_BODY);
                                
                                info!("HPS: Looking for URI characteristic (0x{:04X})...", HpsUuids::URI);
                                let uri_char = match gatt_client.characteristic_by_uuid::<u8>(&service, &uri_uuid).await {
                                    Ok(c) => c,
                                    Err(e) => {
                                        warn!("HPS: URI characteristic not found: {:?}", Debug2Format(&e));
                                        drop(conn);
                                        return;
                                    }
                                };
                                
                                info!("HPS: Looking for Control Point characteristic (0x{:04X})...", HpsUuids::HTTP_CONTROL_POINT);
                                let control_char = match gatt_client.characteristic_by_uuid::<u8>(&service, &control_uuid).await {
                                    Ok(c) => c,
                                    Err(e) => {
                                        warn!("HPS: Control Point characteristic not found: {:?}", Debug2Format(&e));
                                        drop(conn);
                                        return;
                                    }
                                };
                                
                                info!("HPS: Looking for Status Code characteristic (0x{:04X})...", HpsUuids::HTTP_STATUS_CODE);
                                let status_char = match gatt_client.characteristic_by_uuid::<u8>(&service, &status_uuid).await {
                                    Ok(c) => c,
                                    Err(e) => {
                                        warn!("HPS: Status Code characteristic not found: {:?}", Debug2Format(&e));
                                        drop(conn);
                                        return;
                                    }
                                };
                                
                                info!("HPS: Looking for Entity Body characteristic (0x{:04X})...", HpsUuids::HTTP_ENTITY_BODY);
                                let body_char = match gatt_client.characteristic_by_uuid::<u8>(&service, &body_uuid).await {
                                    Ok(c) => c,
                                    Err(e) => {
                                        warn!("HPS: Entity Body characteristic not found: {:?}", Debug2Format(&e));
                                        drop(conn);
                                        return;
                                    }
                                };
                                
                                info!("HPS: All characteristics discovered successfully");
                                
                                // Signal ready
                                info!("HPS: Service discovery complete, signaling ready");
                                HPS_READY_CHANNEL.sender().try_send(true).ok();
                                
                                // Keep connection alive and handle requests
                                loop {
                                    if let Ok(request) = request_rx.try_receive() {
                                        info!("HPS: Received {} request to {}", request.method, request.uri);
                                        
                                        // Execute HPS request via GATT
                                        let response = execute_hps_request(
                                            &gatt_client,
                                            &uri_char,
                                            &control_char,
                                            &status_char,
                                            &body_char,
                                            request
                                        ).await;
                                        
                                        let _ = response_tx.send(response).await;
                                    } else {
                                        Timer::after(Duration::from_millis(100)).await;
                                    }
                                }
                            }).await;
                            
                            // Connection dropped or error occurred
                            warn!("HPS: Connection lost");
                            Timer::after(Duration::from_secs(5)).await;
                            state = HpsServiceState::Disconnected;
                        }
                        Ok(Err(e)) => {
                            warn!("HPS: Connection failed: {:?}", Debug2Format(&e));
                            Timer::after(Duration::from_secs(5)).await;
                            state = HpsServiceState::Disconnected;
                        }
                        Err(_timeout) => {
                            warn!("HPS: Connection timeout after 25s (scan timeout: 20s)");
                            Timer::after(Duration::from_secs(5)).await;
                            state = HpsServiceState::Disconnected;
                        }
                    }
                }
                
                HpsServiceState::Ready => {
                    // This state is now handled inside the Connecting state loop
                    // Should not reach here normally
                    warn!("HPS: Unexpected Ready state, returning to Disconnected");
                    state = HpsServiceState::Disconnected;
                }
                
                HpsServiceState::Error => {
                    warn!("HPS: Error state, attempting recovery...");
                    Timer::after(Duration::from_secs(5)).await;
                    state = HpsServiceState::Disconnected;
                }
            }
        }
    };
    
    // Run both tasks concurrently
    join(runner_task, service_task).await;
}

/// Process an HPS request through GATT with client and characteristics
async fn process_hps_request_with_gatt<T: Controller, P: PacketPool>(
    gatt: &mut GattClient<'_, T, P, 10>,
    chars: &HpsCharacteristics,
    request: HpsRequest,
) -> Result<HttpResponse, HpsError> {
    info!("HPS: Processing HTTP {} request to {}", request.method, request.uri);
    
    // Step 1: Write URI characteristic
    let uri_handle = chars.uri.ok_or(HpsError::NotFound)?;
    info!("HPS: Writing URI ({} bytes)", request.uri.len());
    
    // Convert heapless::String to &[u8]
    let uri_bytes = request.uri.as_bytes();
    gatt_write_raw(gatt, uri_handle, uri_bytes).await?;
    
    // Step 2: Write HTTP Headers characteristic (if non-empty)
    if !request.headers.is_empty() {
        let headers_handle = chars.headers.ok_or(HpsError::NotFound)?;
        info!("HPS: Writing Headers ({} bytes)", request.headers.len());
        let headers_bytes = request.headers.as_bytes();
        gatt_write_raw(gatt, headers_handle, headers_bytes).await?;
    }
    
    // Step 3: Write HTTP Entity Body characteristic (if non-empty)
    if !request.body.is_empty() {
        let body_handle = chars.entity_body.ok_or(HpsError::NotFound)?;
        info!("HPS: Writing Body ({} bytes)", request.body.len());
        gatt_write_raw(gatt, body_handle, &request.body).await?;
    }
    
    // Step 4: Enable notifications on HTTP Status Code characteristic (CCCD)
    let cccd_handle = chars.status_code_cccd.ok_or(HpsError::NotFound)?;
    info!("HPS: Enabling notifications on CCCD");
    // CCCD value: 0x0001 for notifications, little-endian
    gatt_write_raw(gatt, cccd_handle, &[0x01, 0x00]).await?;
    
    // Step 5: Write HTTP Control Point to initiate request
    let control_handle = chars.control_point.ok_or(HpsError::NotFound)?;
    let method_opcode = request.method as u8;
    info!("HPS: Writing Control Point (opcode={})", method_opcode);
    gatt_write_raw(gatt, control_handle, &[method_opcode]).await?;
    
    // Step 6: Wait for HTTP Status Code notification
    // TODO: Implement proper notification waiting mechanism
    // For now, we'll just read the status code directly after a delay
    info!("HPS: Waiting for response...");
    Timer::after(Duration::from_secs(2)).await;
    
    let status_handle = chars.status_code.unwrap_or(0);
    let mut status_buf = [0u8; 3];
    gatt_read_raw(gatt, status_handle, &mut status_buf).await?;
    
    let status_code = HttpStatusCode::from_bytes(&status_buf).ok_or(HpsError::BleError)?;
    info!("HPS: Got status code: {}", status_code.status_code);
    
    // Step 7: Read HTTP Headers if available
    let headers_handle = chars.headers.ok_or(HpsError::NotFound)?;
    let mut headers_buf = [0u8; MAX_HEADERS_SIZE];
    match gatt_read_raw(gatt, headers_handle, &mut headers_buf).await {
        Ok(len) => {
            let headers_str = core::str::from_utf8(&headers_buf[..len])
                .map_err(|_| HpsError::EncodingError)?;
            
            // Step 8: Read HTTP Entity Body if available
            let body_handle = chars.entity_body.ok_or(HpsError::NotFound)?;
            let mut body_buf = [0u8; MAX_BODY_SIZE];
            let body_len = gatt_read_raw(gatt, body_handle, &mut body_buf).await.unwrap_or(0);
            
            info!("HPS: Read {} bytes of response body", body_len);
            
            Ok(HttpResponse {
                status_code: status_code.status_code,
                data_status: status_code.data_status,
                headers: heapless::String::try_from(headers_str).unwrap_or_default(),
                body: heapless::Vec::from_slice(&body_buf[..body_len]).unwrap_or_default(),
            })
        }
        Err(_) => {
            // No headers, try reading body only
            let body_handle = chars.entity_body.ok_or(HpsError::NotFound)?;
            let mut body_buf = [0u8; MAX_BODY_SIZE];
            let body_len = gatt_read_raw(gatt, body_handle, &mut body_buf).await.unwrap_or(0);
            
            info!("HPS: Read {} bytes of response body", body_len);
            
            Ok(HttpResponse {
                status_code: status_code.status_code,
                data_status: status_code.data_status,
                headers: heapless::String::new(),
                body: heapless::Vec::from_slice(&body_buf[..body_len]).unwrap_or_default(),
            })
        }
    }
}

/// Write raw bytes to a GATT characteristic using handle
/// 
/// This function uses unsafe transmute to construct a Characteristic<T> wrapper
/// around a raw handle. This is necessary because trouble-host GattClient methods
/// require Characteristic objects, but service discovery returns them, while we need
/// to store just handles for later use.
async fn gatt_write_raw<T: Controller, P: PacketPool>(
    gatt: &GattClient<'_, T, P, 10>,
    handle: u16,
    data: &[u8],
) -> Result<(), HpsError> {
    use core::marker::PhantomData;
    use core::mem;
    
    // Create a temporary Characteristic manually
    // This is safe because PhantomData is zero-sized and we're just wrapping a handle
    #[repr(C)]
    struct CharWrapper {
        cccd_handle: Option<u16>,
        handle: u16,
        _phantom: PhantomData<Vec<u8, 512>>,
    }
    
    let wrapper = CharWrapper {
        cccd_handle: None,
        handle,
        _phantom: PhantomData,
    };
    
    // Transmute to Characteristic - safe because layout is identical
    let char_handle: &Characteristic<Vec<u8, 512>> = unsafe { mem::transmute(&wrapper) };
    
    gatt.write_characteristic(char_handle, data)
        .await
        .map_err(|_| HpsError::BleError)?;
    
    Ok(())
}

/// Read raw bytes from a GATT characteristic using handle
/// 
/// Similar to gatt_write_raw, uses unsafe transmute to work with raw handles.
async fn gatt_read_raw<T: Controller, P: PacketPool>(
    gatt: &GattClient<'_, T, P, 10>,
    handle: u16,
    buf: &mut [u8],
) -> Result<usize, HpsError> {
    use core::marker::PhantomData;
    use core::mem;
    
    #[repr(C)]
    struct CharWrapper {
        cccd_handle: Option<u16>,
        handle: u16,
        _phantom: PhantomData<Vec<u8, 512>>,
    }
    
    let wrapper = CharWrapper {
        cccd_handle: None,
        handle,
        _phantom: PhantomData,
    };
    
    let char_handle: &Characteristic<Vec<u8, 512>> = unsafe { mem::transmute(&wrapper) };
    
    let len = gatt.read_characteristic(char_handle, buf)
        .await
        .map_err(|_| HpsError::BleError)?;
    
    Ok(len)
}

/// Discover HPS service and all characteristics
/// 
/// Finds the HPS service by UUID (0x1823) and discovers all 6 mandatory characteristics:
/// - URI (0x2AB6)
/// - HTTP Headers (0x2AB7)
/// - HTTP Status Code (0x2AB8) with CCCD for notifications
/// - HTTP Entity Body (0x2AB9)
/// - HTTP Control Point (0x2ABA)
/// - HTTPS Security (0x2ABB)
async fn discover_hps_service<T: Controller, P: PacketPool, const MAX_SERVICES: usize>(
    gatt: &mut GattClient<'_, T, P, MAX_SERVICES>,
) -> Result<HpsCharacteristics, HpsError> {
    info!("HPS: Discovering service and characteristics");
    
    // Find HPS service by UUID (0x1823)
    let hps_uuid = Uuid::Uuid16(HpsUuids::SERVICE.to_le_bytes());
    
    let services = gatt.services_by_uuid(&hps_uuid)
        .await
        .map_err(|_| HpsError::BleError)?;
    
    if services.is_empty() {
        warn!("HPS: Service not found");
        return Err(HpsError::NotFound);
    }
    
    let service = &services[0];
    info!("HPS: Found HPS service");
    
    // Discover all 6 mandatory characteristics
    let mut chars = HpsCharacteristics::default();
    
    // 1. URI Characteristic (0x2AB6)
    let uri_uuid = Uuid::Uuid16(HpsUuids::URI.to_le_bytes());
    match gatt.characteristic_by_uuid::<Vec<u8, 512>>(&service, &uri_uuid).await {
        Ok(char) => {
            chars.uri = Some(char.handle);
            info!("HPS: Found URI characteristic (handle={})", char.handle);
        }
        Err(_) => {
            warn!("HPS: URI characteristic not found (mandatory)");
            return Err(HpsError::NotFound);
        }
    }
    
    // 2. HTTP Headers Characteristic (0x2AB7)
    let headers_uuid = Uuid::Uuid16(HpsUuids::HTTP_HEADERS.to_le_bytes());
    match gatt.characteristic_by_uuid::<Vec<u8, 512>>(&service, &headers_uuid).await {
        Ok(char) => {
            chars.headers = Some(char.handle);
            info!("HPS: Found HTTP Headers characteristic (handle={})", char.handle);
        }
        Err(_) => {
            warn!("HPS: HTTP Headers characteristic not found (mandatory)");
            return Err(HpsError::NotFound);
        }
    }
    
    // 3. HTTP Status Code Characteristic (0x2AB8)
    let status_uuid = Uuid::Uuid16(HpsUuids::HTTP_STATUS_CODE.to_le_bytes());
    match gatt.characteristic_by_uuid::<[u8; 3]>(&service, &status_uuid).await {
        Ok(char) => {
            chars.status_code = Some(char.handle);
            // Get CCCD handle for notifications
            chars.status_code_cccd = char.cccd_handle;
            info!("HPS: Found HTTP Status Code characteristic (handle={}, CCCD={:?})", 
                  char.handle, char.cccd_handle);
        }
        Err(_) => {
            warn!("HPS: HTTP Status Code characteristic not found (mandatory)");
            return Err(HpsError::NotFound);
        }
    }
    
    // 4. HTTP Entity Body Characteristic (0x2AB9)
    let body_uuid = Uuid::Uuid16(HpsUuids::HTTP_ENTITY_BODY.to_le_bytes());
    match gatt.characteristic_by_uuid::<Vec<u8, 512>>(&service, &body_uuid).await {
        Ok(char) => {
            chars.entity_body = Some(char.handle);
            info!("HPS: Found HTTP Entity Body characteristic (handle={})", char.handle);
        }
        Err(_) => {
            warn!("HPS: HTTP Entity Body characteristic not found (mandatory)");
            return Err(HpsError::NotFound);
        }
    }
    
    // 5. HTTP Control Point Characteristic (0x2ABA)
    let control_uuid = Uuid::Uuid16(HpsUuids::HTTP_CONTROL_POINT.to_le_bytes());
    match gatt.characteristic_by_uuid::<u8>(&service, &control_uuid).await {
        Ok(char) => {
            chars.control_point = Some(char.handle);
            info!("HPS: Found HTTP Control Point characteristic (handle={})", char.handle);
        }
        Err(_) => {
            warn!("HPS: HTTP Control Point characteristic not found (mandatory)");
            return Err(HpsError::NotFound);
        }
    }
    
    // 6. HTTPS Security Characteristic (0x2ABB)
    let security_uuid = Uuid::Uuid16(HpsUuids::HTTPS_SECURITY.to_le_bytes());
    match gatt.characteristic_by_uuid::<u8>(&service, &security_uuid).await {
        Ok(char) => {
            chars.https_security = Some(char.handle);
            info!("HPS: Found HTTPS Security characteristic (handle={})", char.handle);
        }
        Err(_) => {
            warn!("HPS: HTTPS Security characteristic not found (mandatory)");
            return Err(HpsError::NotFound);
        }
    }
    
    // Verify all characteristics were found
    if !chars.is_complete() {
        warn!("HPS: Not all mandatory characteristics found");
        return Err(HpsError::NotFound);
    }
    
    info!("HPS: Successfully discovered all HPS characteristics");
    Ok(chars)
}

/// Parse service UUIDs from advertising data
/// Returns true if HPS service UUID (0x1823) is found
fn has_hps_service(ad_data: &[u8]) -> bool {
    let mut offset = 0;
    let target_uuid = HpsUuids::SERVICE.to_le_bytes();
    
    while offset < ad_data.len() {
        let len = ad_data[offset] as usize;
        if len == 0 {
            break;
        }
        
        let segment_end = offset + 1 + len;
        if segment_end > ad_data.len() {
            break;
        }
        
        let ad_type = ad_data[offset + 1];
        
        // 0x02 = Incomplete List of 16-bit Service UUIDs
        // 0x03 = Complete List of 16-bit Service UUIDs
        if matches!(ad_type, 0x02 | 0x03) {
            let uuid_data = &ad_data[(offset + 2)..segment_end];
            
            // Each 16-bit UUID is 2 bytes
            for chunk in uuid_data.chunks(2) {
                if chunk.len() == 2 && chunk == target_uuid {
                    return true;
                }
            }
        }
        
        offset = segment_end;
    }
    
    false
}

/// Event handler for HPS scanning (for future dynamic discovery)
/// Currently unused since we use direct connection with configured address
struct HpsScanHandler {
    found_addr: RefCell<Option<[u8; 6]>>,
}

impl HpsScanHandler {
    fn new() -> Self {
        Self {
            found_addr: RefCell::new(None),
        }
    }
    
    fn take_found_device(&self) -> Option<[u8; 6]> {
        self.found_addr.borrow_mut().take()
    }
    
    fn handle_report(&self, report: &LeAdvReport<'_>) {
        // Skip if already found HPS device
        if self.found_addr.borrow().is_some() {
            return;
        }
        
        // Check if this device advertises HPS service
        if has_hps_service(report.data) {
            let mut addr = [0u8; 6];
            addr.copy_from_slice(report.addr.raw());
            info!("HPS: Found HPS server at {=[u8]:02X}", addr);
            *self.found_addr.borrow_mut() = Some(addr);
        }
    }
}

impl EventHandler for HpsScanHandler {
    fn on_adv_reports(&self, mut reports: LeAdvReportsIter<'_>) {
        while let Some(Ok(report)) = reports.next() {
            self.handle_report(&report);
        }
    }
}

/// Execute an HPS request via GATT characteristics
async fn execute_hps_request<C: Controller, P: PacketPool>(
    gatt: &GattClient<'_, C, P, 10>,
    uri_char: &Characteristic<u8>,
    control_char: &Characteristic<u8>,
    status_char: &Characteristic<u8>,
    body_char: &Characteristic<u8>,
    request: HpsRequest,
) -> HpsResponse {
    info!("HPS: Writing URI: {}", request.uri);
    
    // Write URI
    let uri_bytes = request.uri.as_bytes();
    if let Err(e) = gatt.write_characteristic(uri_char, uri_bytes).await {
        warn!("HPS: Failed to write URI: {:?}", Debug2Format(&e));
        return Err(HpsError::BleError);
    }
    
    info!("HPS: URI written successfully");
    
    // Write Control Point to trigger request
    let method_opcode = request.method as u8;
    info!("HPS: Writing Control Point (method opcode: 0x{:02X})...", method_opcode);
    
    if let Err(e) = gatt.write_characteristic(control_char, &[method_opcode]).await {
        warn!("HPS: Failed to write Control Point: {:?}", Debug2Format(&e));
        return Err(HpsError::BleError);
    }
    
    info!("HPS: Control Point written, waiting for server to process...");
    
    // Wait for server to process the request
    Timer::after(Duration::from_secs(3)).await;
    
    info!("HPS: Reading Status Code...");
    
    // Read Status Code (3 bytes: u16 status + u8 data_status)
    let mut status_buf = [0u8; 3];
    if let Err(e) = gatt.read_characteristic(status_char, &mut status_buf).await {
        warn!("HPS: Failed to read Status Code: {:?}", Debug2Format(&e));
        return Err(HpsError::BleError);
    }
    
    let status_code = u16::from_le_bytes([status_buf[0], status_buf[1]]);
    let data_status_byte = status_buf[2];
    
    info!("HPS: Status Code: {}, Data Status: 0x{:02X}", status_code, data_status_byte);
    
    // Read Response Body
    info!("HPS: Reading Entity Body...");
    let mut body_buf = [0u8; 512];
    let body_len = match gatt.read_characteristic(body_char, &mut body_buf).await {
        Ok(len) => len,
        Err(e) => {
            warn!("HPS: Failed to read Entity Body: {:?}", Debug2Format(&e));
            0
        }
    };
    
    info!("HPS: Read {} bytes of response body", body_len);
    
    // Build response
    let mut body = Vec::new();
    let _ = body.extend_from_slice(&body_buf[..body_len]);
    
    // Parse data_status
    let data_status = DataStatus {
        headers_received: (data_status_byte & 0x01) != 0,
        headers_truncated: (data_status_byte & 0x02) != 0,
        body_received: (data_status_byte & 0x04) != 0,
        body_truncated: (data_status_byte & 0x08) != 0,
    };
    
    Ok(HttpResponse {
        status_code: status_code as u16,
        data_status,
        headers: heapless::String::new(),
        body,
    })
}
