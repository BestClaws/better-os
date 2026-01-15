// HPS (HTTP Proxy Service) Client Service
// Manages BLE connection to HPS server and handles HTTP requests via GATT

use bt_hci::param::LeAdvReport;
use defmt::{info, warn, Debug2Format};
use embassy_futures::join::join;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::channel::{Channel, Receiver, Sender};
use embassy_sync::mutex::Mutex;
use embassy_time::{with_timeout, Duration, Instant, Timer};
use trouble_host::prelude::*;
use trouble_host::types::gatt_traits::{AsGatt, FromGatt, FromGattError};
#[derive(Clone, Copy, Debug, Default)]
struct GattBuffer<const N: usize>;

impl<const N: usize> AsGatt for GattBuffer<N> {
    const MIN_SIZE: usize = 0;
    const MAX_SIZE: usize = N;

    fn as_gatt(&self) -> &[u8] {
        &[]
    }
}

impl<const N: usize> FromGatt for GattBuffer<N> {
    fn from_gatt(data: &[u8]) -> Result<Self, FromGattError> {
        if data.len() <= N {
            Ok(Self)
        } else {
            Err(FromGattError::InvalidLength)
        }
    }
}

use crate::libs::hps::error::HpsError;
use crate::libs::hps::types::{
    DataStatus, HpsCharacteristics, HpsUuids, HttpMethod, HttpRequest, HttpResponse,
    HttpStatusCode, MAX_BODY_SIZE, MAX_HEADERS_SIZE, MAX_URI_SIZE,
};
use crate::system::hal::radio::AsyncRadio;
use alloc::boxed::Box;
use alloc::string::String;
use alloc::vec::Vec;
use core::cell::RefCell;

const STATUS_POLL_INTERVAL_MS: u64 = 120;
const STATUS_WAIT_TIMEOUT_MS: u64 = 5_000;
const SLOW_STAGE_LOG_THRESHOLD_MS: u64 = 250;

/// HPS request message (heap-allocated to avoid stack overflow)
#[derive(Debug)]
pub struct HpsRequest {
    pub method: HttpMethod,
    pub uri: String,
    pub headers: String,
    pub body: Vec<u8>,
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
    info!("HPS: Acquiring radio lock...");
    let mut _radio_guard = radio.lock().await;
    info!("HPS: Radio lock acquired, getting stack...");
    let stack = _radio_guard.get_stack().await;
    info!("HPS: Stack acquired, building host...");

    // Build host components - this borrows stack, doesn't consume it
    let Host {
        mut central,
        peripheral: _peripheral,
        mut runner,
        ..
    } = stack.build();

    info!("HPS: Host built successfully");

    let request_rx = HPS_REQUEST_CHANNEL.receiver();
    let response_tx = HPS_RESPONSE_CHANNEL.sender();

    // Create event handler for scanning
    let handler = HpsScanHandler::new();

    // Run BLE stack with event handler to receive scan results
    info!("HPS: Launching BLE host runner...");
    let runner_task = async {
        info!("HPS: BLE host runner active; processing controller events");
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
        scan_config.filter_accept_list = &[]; // Scan for any device

        let (target_kind, target_addr) = loop {
            handler.prepare_for_scan();
            info!("HPS: Scanning up to 10s for HTTP Proxy advertisements...");
            match scanner.scan(&scan_config).await {
                Ok(_session) => {
                    Timer::after(Duration::from_secs(10)).await;
                    info!("HPS: Scan finished; compiling advertisement list...");
                }
                Err(e) => {
                    warn!("HPS: Failed to start scan: {:?}", Debug2Format(&e));
                }
            }

            let observations = handler.snapshot_results();
            if observations.is_empty() {
                info!("HPS: Scan observed no advertising devices");
            } else {
                info!("HPS: Scan observed {} advertising devices", observations.len());
                for entry in &observations {
                    let name = entry
                        .name
                        .as_deref()
                        .unwrap_or("(unknown)");
                    let addr_kind = describe_addr_kind(entry.addr_kind);
                    if entry.has_hps {
                        info!(
                            "HPS:   [HTTP Proxy] {} ({}) @ {=[u8]:02X}",
                            name,
                            addr_kind,
                            entry.addr
                        );
                    } else {
                        info!(
                            "HPS:   [generic] {} ({}) @ {=[u8]:02X}",
                            name,
                            addr_kind,
                            entry.addr
                        );
                    }
                }
            }

            if let Some((addr_kind, addr)) = handler.take_found_device() {
                info!(
                    "HPS: Selecting HTTP Proxy peripheral at {=[u8]:02X} ({})",
                    addr,
                    describe_addr_kind(addr_kind)
                );
                break (addr_kind, addr);
            }

            warn!("HPS: HTTP Proxy service not observed during scan window; retrying after 2s...");
            Timer::after(Duration::from_secs(2)).await;
        };

        let target = Address {
            kind: target_kind,
            addr: BdAddr::new(target_addr),
        };

        info!(
            "HPS: Targeting {} address {=[u8]:02X}",
            describe_addr_kind(target_kind),
            target_addr
        );

        // Get central back for connection
        let mut central = scanner.into_inner();

        loop {
            match state {
                HpsServiceState::Disconnected => {
                    info!("HPS: Link idle; scheduling reconnect attempt in 2s...");
                    Timer::after(Duration::from_secs(2)).await;
                    state = HpsServiceState::Connecting;
                }

                HpsServiceState::Connecting => {
                    info!(
                        "HPS: Initiating BLE link with HTTP Proxy at {=[u8]:02X} ({})",
                        target_addr,
                        describe_addr_kind(target_kind)
                    );
                    info!("HPS: Using {} address type; scanning until peer responds", describe_addr_kind(target_kind));

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
                            window: Duration::from_millis(100), // 100% duty cycle
                            timeout: Duration::from_secs(20),
                        },
                    };

                    info!("HPS: Awaiting connection (scan timeout 20s, overall 25s)");
                    // Wrap in timeout to ensure we don't hang forever
                    let result =
                        with_timeout(Duration::from_secs(25), central.connect(&config)).await;

                    match result {
                        Ok(Ok(conn)) => {
                            info!("HPS: Link established; preparing GATT client session");

                            // Create GATT client - type inference will figure out the types
                            let gatt_client = match GattClient::<_, _, 10>::new(&stack, &conn).await
                            {
                                Ok(client) => client,
                                Err(e) => {
                                    warn!(
                                        "HPS: Failed to create GATT client: {:?}",
                                        Debug2Format(&e)
                                    );
                                    drop(conn);
                                    Timer::after(Duration::from_secs(5)).await;
                                    state = HpsServiceState::Disconnected;
                                    continue;
                                }
                            };

                            // Run GATT client task concurrently with service discovery and operations
                            info!("HPS: Managing GATT session and HTTP Proxy discovery");
                            let _ = join(
                                async {
                                    info!("HPS: ATT event pump active for connected peer");
                                    let result = gatt_client.task().await;
                                    warn!("HPS: GATT client task ended: {:?}", Debug2Format(&result));
                                    result
                                },
                                async {
                                    // Give GATT client task time to initialize
                                    info!("HPS: Allowing peer to settle before discovery (2s grace period)...");
                                    Timer::after(Duration::from_secs(2)).await;

                                    info!("HPS: Discovering HTTP Proxy service (UUID 0x1823)...");
                                    let hps_uuid = Uuid::new_short(0x1823);

                                    info!("HPS: Querying remote GATT database for service definition (timeout 15s)...");
                                    let services_result = with_timeout(
                                        Duration::from_secs(15),
                                        gatt_client.services_by_uuid(&hps_uuid)
                                    ).await;

                                let services = match services_result {
                                    Ok(Ok(svcs)) => {
                                        info!("HPS: HTTP Proxy service present; {} matching entries discovered", svcs.len());
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
                                info!("HPS: Inspecting HTTP Proxy characteristic set for selected service...");

                                // Discover HPS characteristics
                                let uri_uuid = Uuid::new_short(HpsUuids::URI);
                                let headers_uuid = Uuid::new_short(HpsUuids::HTTP_HEADERS);
                                let control_uuid = Uuid::new_short(HpsUuids::HTTP_CONTROL_POINT);
                                let status_uuid = Uuid::new_short(HpsUuids::HTTP_STATUS_CODE);
                                let body_uuid = Uuid::new_short(HpsUuids::HTTP_ENTITY_BODY);

                                info!("HPS: Resolving URI characteristic (0x{:04X}) for request paths...", HpsUuids::URI);
                                let uri_char = match gatt_client.characteristic_by_uuid::<u8>(&service, &uri_uuid).await {
                                    Ok(c) => c,
                                    Err(e) => {
                                        warn!("HPS: URI characteristic not found: {:?}", Debug2Format(&e));
                                        drop(conn);
                                        return;
                                    }
                                };

                                info!("HPS: Resolving Headers characteristic (0x{:04X}) for outbound metadata...", HpsUuids::HTTP_HEADERS);
                                let headers_char = match gatt_client.characteristic_by_uuid::<u8>(&service, &headers_uuid).await {
                                    Ok(c) => c,
                                    Err(e) => {
                                        warn!("HPS: Headers characteristic not found: {:?}", Debug2Format(&e));
                                        drop(conn);
                                        return;
                                    }
                                };

                                info!("HPS: Resolving Control Point characteristic (0x{:04X}) for method dispatch...", HpsUuids::HTTP_CONTROL_POINT);
                                let control_char = match gatt_client.characteristic_by_uuid::<u8>(&service, &control_uuid).await {
                                    Ok(c) => c,
                                    Err(e) => {
                                        warn!("HPS: Control Point characteristic not found: {:?}", Debug2Format(&e));
                                        drop(conn);
                                        return;
                                    }
                                };

                                info!("HPS: Resolving Status Code characteristic (0x{:04X}) for response status...", HpsUuids::HTTP_STATUS_CODE);
                                let status_char = match gatt_client.characteristic_by_uuid::<u8>(&service, &status_uuid).await {
                                    Ok(c) => c,
                                    Err(e) => {
                                        warn!("HPS: Status Code characteristic not found: {:?}", Debug2Format(&e));
                                        drop(conn);
                                        return;
                                    }
                                };

                                info!("HPS: Resolving Entity Body characteristic (0x{:04X}) for payload transfer...", HpsUuids::HTTP_ENTITY_BODY);
                                let body_char = match gatt_client.characteristic_by_uuid::<u8>(&service, &body_uuid).await {
                                    Ok(c) => c,
                                    Err(e) => {
                                        warn!("HPS: Entity Body characteristic not found: {:?}", Debug2Format(&e));
                                        drop(conn);
                                        return;
                                    }
                                };
                                info!("HPS: Characteristic map ready (URI, headers, control point, status, body)");

                                // Signal ready
                                info!("HPS: HTTP Proxy link ready; notifying waiters");
                                HPS_READY_CHANNEL.sender().try_send(true).ok();

                                // Keep connection alive and handle requests
                                loop {
                                    if let Ok(request) = request_rx.try_receive() {
                                        info!("HPS: Received {} request to {}", request.method, request.uri.as_str());

                                        // Execute HPS request via GATT (keep boxed to avoid stack overflow)
                                        let response = execute_hps_request(
                                            &gatt_client,
                                            &uri_char,
                                            &headers_char,
                                            &control_char,
                                            &status_char,
                                            &body_char,
                                            request  // Pass Box, extract fields inside
                                        ).await;

                                        let _ = response_tx.send(response).await;
                                    } else {
                                        Timer::after(Duration::from_millis(100)).await;
                                    }
                                }
                            }).await;

                            // Connection dropped or error occurred
                            warn!("HPS: Peer disconnected; retrying after backoff");
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
    info!(
        "HPS: Processing HTTP {} request to {}",
        request.method,
        request.uri.as_str()
    );

    // Step 1: Write URI characteristic
    let uri_handle = chars.uri.ok_or(HpsError::NotFound)?;
    info!("HPS: Writing URI ({} bytes)", request.uri.len());

    // Convert String to &[u8]
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
            let headers_str =
                core::str::from_utf8(&headers_buf[..len]).map_err(|_| HpsError::EncodingError)?;

            // Step 8: Read HTTP Entity Body if available
            let body_handle = chars.entity_body.ok_or(HpsError::NotFound)?;
            let mut body_buf = [0u8; MAX_BODY_SIZE];
            let body_len = gatt_read_raw(gatt, body_handle, &mut body_buf)
                .await
                .unwrap_or(0);

            info!("HPS: Read {} bytes of response body", body_len);

            Ok(HttpResponse {
                status_code: status_code.status_code,
                data_status: status_code.data_status,
                headers: String::from(headers_str),
                body: body_buf[..body_len].to_vec(),
            })
        }
        Err(_) => {
            // No headers, try reading body only
            let body_handle = chars.entity_body.ok_or(HpsError::NotFound)?;
            let mut body_buf = [0u8; MAX_BODY_SIZE];
            let body_len = gatt_read_raw(gatt, body_handle, &mut body_buf)
                .await
                .unwrap_or(0);

            info!("HPS: Read {} bytes of response body", body_len);

            Ok(HttpResponse {
                status_code: status_code.status_code,
                data_status: status_code.data_status,
                headers: String::new(),
                body: body_buf[..body_len].to_vec(),
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
        _phantom: PhantomData<GattBuffer<512>>,
    }

    let wrapper = CharWrapper {
        cccd_handle: None,
        handle,
        _phantom: PhantomData,
    };

    // Transmute to Characteristic - safe because layout is identical
    let char_handle: &Characteristic<GattBuffer<512>> = unsafe { mem::transmute(&wrapper) };

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

    // Use 1024 bytes - reasonable size that won't cause memory issues
    // For larger data, we'll need multiple reads
    #[repr(C)]
    struct CharWrapper {
        cccd_handle: Option<u16>,
        handle: u16,
        _phantom: PhantomData<GattBuffer<1024>>,
    }

    let wrapper = CharWrapper {
        cccd_handle: None,
        handle,
        _phantom: PhantomData,
    };

    let char_handle: &Characteristic<GattBuffer<1024>> = unsafe { mem::transmute(&wrapper) };

    let len = gatt
        .read_characteristic(char_handle, buf)
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

    let services = gatt
        .services_by_uuid(&hps_uuid)
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
    match gatt
        .characteristic_by_uuid::<GattBuffer<512>>(&service, &uri_uuid)
        .await
    {
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
    match gatt
        .characteristic_by_uuid::<GattBuffer<512>>(&service, &headers_uuid)
        .await
    {
        Ok(char) => {
            chars.headers = Some(char.handle);
            info!(
                "HPS: Found HTTP Headers characteristic (handle={})",
                char.handle
            );
        }
        Err(_) => {
            warn!("HPS: HTTP Headers characteristic not found (mandatory)");
            return Err(HpsError::NotFound);
        }
    }

    // 3. HTTP Status Code Characteristic (0x2AB8)
    let status_uuid = Uuid::Uuid16(HpsUuids::HTTP_STATUS_CODE.to_le_bytes());
    match gatt
        .characteristic_by_uuid::<[u8; 3]>(&service, &status_uuid)
        .await
    {
        Ok(char) => {
            chars.status_code = Some(char.handle);
            // Get CCCD handle for notifications
            chars.status_code_cccd = char.cccd_handle;
            info!(
                "HPS: Found HTTP Status Code characteristic (handle={}, CCCD={:?})",
                char.handle, char.cccd_handle
            );
        }
        Err(_) => {
            warn!("HPS: HTTP Status Code characteristic not found (mandatory)");
            return Err(HpsError::NotFound);
        }
    }

    // 4. HTTP Entity Body Characteristic (0x2AB9)
    let body_uuid = Uuid::Uuid16(HpsUuids::HTTP_ENTITY_BODY.to_le_bytes());
    match gatt
        .characteristic_by_uuid::<GattBuffer<512>>(&service, &body_uuid)
        .await
    {
        Ok(char) => {
            chars.entity_body = Some(char.handle);
            info!(
                "HPS: Found HTTP Entity Body characteristic (handle={})",
                char.handle
            );
        }
        Err(_) => {
            warn!("HPS: HTTP Entity Body characteristic not found (mandatory)");
            return Err(HpsError::NotFound);
        }
    }

    // 5. HTTP Control Point Characteristic (0x2ABA)
    let control_uuid = Uuid::Uuid16(HpsUuids::HTTP_CONTROL_POINT.to_le_bytes());
    match gatt
        .characteristic_by_uuid::<u8>(&service, &control_uuid)
        .await
    {
        Ok(char) => {
            chars.control_point = Some(char.handle);
            info!(
                "HPS: Found HTTP Control Point characteristic (handle={})",
                char.handle
            );
        }
        Err(_) => {
            warn!("HPS: HTTP Control Point characteristic not found (mandatory)");
            return Err(HpsError::NotFound);
        }
    }

    // 6. HTTPS Security Characteristic (0x2ABB)
    let security_uuid = Uuid::Uuid16(HpsUuids::HTTPS_SECURITY.to_le_bytes());
    match gatt
        .characteristic_by_uuid::<u8>(&service, &security_uuid)
        .await
    {
        Ok(char) => {
            chars.https_security = Some(char.handle);
            info!(
                "HPS: Found HTTPS Security characteristic (handle={})",
                char.handle
            );
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

fn describe_addr_kind(kind: AddrKind) -> &'static str {
    if kind == AddrKind::PUBLIC {
        "public"
    } else if kind == AddrKind::RANDOM {
        "random"
    } else if kind == AddrKind::RESOLVABLE_PRIVATE_OR_PUBLIC {
        "resolvable/public"
    } else if kind == AddrKind::RESOLVABLE_PRIVATE_OR_RANDOM {
        "resolvable/random"
    } else if kind == AddrKind::ANONYMOUS_ADV {
        "anonymous"
    } else {
        "unknown"
    }
}

/// Extract device name (short or complete) from advertising data
fn extract_device_name(ad_data: &[u8]) -> Option<String> {
    let mut offset = 0;

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
        if matches!(ad_type, 0x08 | 0x09) {
            let name_data = &ad_data[(offset + 2)..segment_end];
            if let Ok(name_str) = core::str::from_utf8(name_data) {
                return Some(String::from(name_str));
            }
        }

        offset = segment_end;
    }

    None
}

#[derive(Clone)]
struct ScanObservation {
    addr: [u8; 6],
    addr_kind: AddrKind,
    name: Option<String>,
    has_hps: bool,
}

/// Event handler for HPS scanning (for future dynamic discovery)
/// Currently unused since we use direct connection with configured address
struct HpsScanHandler {
    found_addr: RefCell<Option<(AddrKind, [u8; 6])>>,
    results: RefCell<Vec<ScanObservation>>,
}

impl HpsScanHandler {
    fn new() -> Self {
        Self {
            found_addr: RefCell::new(None),
            results: RefCell::new(Vec::new()),
        }
    }

    fn prepare_for_scan(&self) {
        if let Ok(mut found) = self.found_addr.try_borrow_mut() {
            *found = None;
        }
        if let Ok(mut results) = self.results.try_borrow_mut() {
            results.clear();
        }
    }

    fn take_found_device(&self) -> Option<(AddrKind, [u8; 6])> {
        self.found_addr.borrow_mut().take()
    }

    fn snapshot_results(&self) -> Vec<ScanObservation> {
        self.results.borrow().clone()
    }

    fn handle_report(&self, report: &LeAdvReport<'_>) {
        // Check if this device advertises HPS service
        let mut addr = [0u8; 6];
        addr.copy_from_slice(report.addr.raw());
        let addr_kind = report.addr_kind;
        let has_hps = has_hps_service(report.data);
        let mut name_opt = extract_device_name(report.data);

        let mut results = self.results.borrow_mut();
        if let Some(existing) = results.iter_mut().find(|entry| entry.addr == addr) {
            existing.addr_kind = addr_kind;
            if let Some(name) = name_opt.take() {
                existing.name = Some(name);
            }
            if has_hps {
                existing.has_hps = true;
            }
        } else {
            results.push(ScanObservation {
                addr,
                addr_kind,
                name: name_opt.take(),
                has_hps,
            });
        }

        if has_hps && self.found_addr.borrow().is_none() {
            info!(
                "HPS: Advertisement from HTTP Proxy candidate at {=[u8]:02X} ({})",
                addr,
                describe_addr_kind(addr_kind)
            );
            *self.found_addr.borrow_mut() = Some((addr_kind, addr));
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
/// Implements full HPS v1.0 protocol with headers and body
async fn execute_hps_request<C: Controller, P: PacketPool>(
    gatt: &GattClient<'_, C, P, 10>,
    uri_char: &Characteristic<u8>,
    headers_char: &Characteristic<u8>,
    control_char: &Characteristic<u8>,
    status_char: &Characteristic<u8>,
    body_char: &Characteristic<u8>,
    request: HpsRequest,
) -> HpsResponse {
    let request_start = Instant::now();
    info!(
        "HPS: Request start method={} uri={} headers={} bytes body={} bytes",
        request.method,
        request.uri.as_str(),
        request.headers.len(),
        request.body.len()
    );

    let method = request.method;
    let uri_bytes = request.uri.as_bytes();
    let headers_bytes = request.headers.as_bytes();
    let body_slice = request.body.as_slice();

    let log_stage_timing = |label: &'static str, elapsed_ms: u64| {
        if elapsed_ms > SLOW_STAGE_LOG_THRESHOLD_MS {
            info!("HPS: {} took {} ms (slow)", label, elapsed_ms);
        } else {
            info!("HPS: {} took {} ms", label, elapsed_ms);
        }
    };

    let stage_start = Instant::now();
    if let Err(e) = gatt.write_characteristic(uri_char, uri_bytes).await {
        warn!("HPS: Failed to write URI: {:?}", Debug2Format(&e));
        return Err(HpsError::BleError);
    }
    log_stage_timing("write_uri", stage_start.elapsed().as_millis());

    if !headers_bytes.is_empty() {
        let stage_start = Instant::now();
        if let Err(e) = gatt.write_characteristic(headers_char, headers_bytes).await {
            warn!("HPS: Failed to write headers: {:?}", Debug2Format(&e));
            return Err(HpsError::BleError);
        }
        log_stage_timing("write_headers", stage_start.elapsed().as_millis());
    } else {
        let stage_start = Instant::now();
        if let Err(e) = gatt.write_characteristic(headers_char, &[]).await {
            warn!("HPS: Failed to write empty headers: {:?}", Debug2Format(&e));
            return Err(HpsError::BleError);
        }
        log_stage_timing("write_headers_empty", stage_start.elapsed().as_millis());
    }

    if !body_slice.is_empty() {
        let stage_start = Instant::now();
        if let Err(e) = gatt.write_characteristic(body_char, body_slice).await {
            warn!("HPS: Failed to write body: {:?}", Debug2Format(&e));
            return Err(HpsError::BleError);
        }
        log_stage_timing("write_body", stage_start.elapsed().as_millis());
    } else {
        let stage_start = Instant::now();
        if let Err(e) = gatt.write_characteristic(body_char, &[]).await {
            warn!("HPS: Failed to write empty body: {:?}", Debug2Format(&e));
            return Err(HpsError::BleError);
        }
        log_stage_timing("write_body_empty", stage_start.elapsed().as_millis());
    }

    let stage_start = Instant::now();
    let method_opcode = method as u8;
    if let Err(e) = gatt
        .write_characteristic(control_char, &[method_opcode])
        .await
    {
        warn!("HPS: Failed to write Control Point: {:?}", Debug2Format(&e));
        return Err(HpsError::BleError);
    }
    log_stage_timing("write_control_point", stage_start.elapsed().as_millis());

    let status_wait_start = Instant::now();
    let status_result = with_timeout(Duration::from_millis(STATUS_WAIT_TIMEOUT_MS), async {
        let mut attempts: u32 = 0;
        loop {
            attempts += 1;
            let mut status_buf = [0u8; 3];
            match gatt.read_characteristic(status_char, &mut status_buf).await {
                Ok(len) if len >= 3 => {
                    let status_code = u16::from_le_bytes([status_buf[0], status_buf[1]]);
                    let data_status = DataStatus::from_byte(status_buf[2]);
                    if status_code != 0 || data_status.headers_received || data_status.body_received
                    {
                        return Ok((status_code, data_status, attempts));
                    }

                    if attempts == 1 {
                        info!("HPS: Status pending (initial read)");
                    } else if attempts % 5 == 0 {
                        info!("HPS: Status still pending after {} polls", attempts);
                    }
                }
                Ok(len) => {
                    warn!("HPS: Status read returned unexpected length {}", len);
                }
                Err(e) => {
                    warn!("HPS: Failed to read Status Code: {:?}", Debug2Format(&e));
                    return Err(HpsError::BleError);
                }
            }

            Timer::after(Duration::from_millis(STATUS_POLL_INTERVAL_MS)).await;
        }
    })
    .await;

    let (status_code, data_status, poll_attempts) = match status_result {
        Ok(Ok(tuple)) => tuple,
        Ok(Err(e)) => return Err(e),
        Err(_) => {
            warn!(
                "HPS: Status wait timed out after {} ms",
                STATUS_WAIT_TIMEOUT_MS
            );
            return Err(HpsError::Timeout);
        }
    };

    let status_wait_ms = status_wait_start.elapsed().as_millis();
    log_stage_timing("wait_status", status_wait_ms);
    if poll_attempts > 1 {
        info!(
            "HPS: Status available after {} polls ({} ms)",
            poll_attempts, status_wait_ms
        );
    }

    info!(
        "HPS: Status {} data_status=0x{:02X}",
        status_code,
        data_status.to_byte()
    );

    let mut headers = String::new();
    if data_status.headers_received {
        let stage_start = Instant::now();
        let mut headers_buf = alloc::vec![0u8; MAX_HEADERS_SIZE];
        match gatt
            .read_characteristic(headers_char, &mut headers_buf)
            .await
        {
            Ok(len) => {
                let elapsed_ms = stage_start.elapsed().as_millis();
                log_stage_timing("read_headers", elapsed_ms);
                info!(
                    "HPS: Read {} bytes of headers{}",
                    len,
                    if data_status.headers_truncated {
                        " (truncated)"
                    } else {
                        ""
                    }
                );
                if let Ok(headers_str) = core::str::from_utf8(&headers_buf[..len]) {
                    headers.push_str(headers_str);
                }
            }
            Err(e) => {
                warn!("HPS: Failed to read headers: {:?}", Debug2Format(&e));
            }
        }
    }

    let mut body = Vec::new();
    if data_status.body_received {
        let stage_start = Instant::now();
        let mut body_buf = alloc::vec![0u8; MAX_BODY_SIZE];
        match gatt.read_characteristic(body_char, &mut body_buf).await {
            Ok(len) => {
                let elapsed_ms = stage_start.elapsed().as_millis();
                log_stage_timing("read_body", elapsed_ms);
                info!(
                    "HPS: Read {} bytes of body{}",
                    len,
                    if data_status.body_truncated {
                        " (truncated)"
                    } else {
                        ""
                    }
                );
                let _ = body.extend_from_slice(&body_buf[..len]);
            }
            Err(e) => {
                warn!("HPS: Failed to read body: {:?}", Debug2Format(&e));
            }
        }
    }

    let total_ms = request_start.elapsed().as_millis();
    info!(
        "HPS: Request complete - status={} headers={} bytes body={} bytes total={} ms",
        status_code,
        headers.len(),
        body.len(),
        total_ms
    );

    Ok(HttpResponse {
        status_code,
        data_status,
        headers,
        body,
    })
}
