mod gatt;
mod scan;
mod types;

pub use types::{
    hps_request_sender, hps_response_receiver, wait_for_hps_ready, HpsRequest, HpsResponse,
};

use alloc::boxed::Box;
use defmt::{info, warn, Debug2Format};
use embassy_futures::join::join;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::mutex::Mutex;
use embassy_time::{with_timeout, Duration, Timer};
use trouble_host::prelude::*;

use crate::system::hal::radio::AsyncRadio;

use gatt::{discover_hps_service, execute_hps_request};
use scan::{describe_addr_kind, HpsScanHandler};
use types::{request_receiver, response_sender, signal_ready};

enum HpsServiceState {
    Disconnected,
    Connecting,
    Ready,
    Error,
}

#[embassy_executor::task]
pub(crate) async fn hps_service(
    radio: &'static Mutex<CriticalSectionRawMutex, Box<dyn AsyncRadio>>,
) {
    info!("HPS service starting");

    info!("HPS: Acquiring radio lock...");
    let mut _radio_guard = radio.lock().await;
    info!("HPS: Radio lock acquired, getting stack...");
    let stack = _radio_guard.get_stack().await;
    info!("HPS: Stack acquired, building host...");

    let Host {
        mut central,
        peripheral: _peripheral,
        mut runner,
        ..
    } = stack.build();

    info!("HPS: Host built successfully");

    let handler = HpsScanHandler::new();

    let runner_task = async {
        info!("HPS: BLE host runner active; processing controller events");
        let result = runner.run_with_handler(&handler).await;
        warn!("HPS: Runner task ended with: {:?}", Debug2Format(&result));
        result
    };

    let service_task = async {
        let mut state = HpsServiceState::Disconnected;
        let mut scanner = Scanner::new(central);

        let mut scan_config = ScanConfig::default();
        scan_config.active = true;
        scan_config.phys = PhySet::M1;
        scan_config.interval = Duration::from_millis(100);
        scan_config.window = Duration::from_millis(100);
        scan_config.timeout = Duration::from_secs(10);
        scan_config.filter_accept_list = &[];

        let (target_kind, target_addr) = loop {
            handler.prepare_for_scan();
            info!("HPS: Scanning up to 10s for HTTP Proxy advertisements...");

            let mut session_active = false;
            match scanner.scan(&scan_config).await {
                Ok(session) => {
                    session_active = true;
                    let mut session = Some(session);
                    let poll_interval = Duration::from_millis(200);
                    let mut elapsed = Duration::from_millis(0);
                    let timeout = scan_config.timeout;

                    while elapsed < timeout {
                        if handler.peek_found_device().is_some() {
                            info!("HPS: HTTP Proxy candidate observed; ending scan early");
                            break;
                        }
                        Timer::after(poll_interval).await;
                        elapsed += poll_interval;
                    }

                    drop(session.take());
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
                info!(
                    "HPS: Scan observed {} advertising devices",
                    observations.len()
                );
                for entry in &observations {
                    let name = entry.name.as_deref().unwrap_or("(unknown)");
                    let addr_kind = describe_addr_kind(entry.addr_kind);
                    if entry.has_hps {
                        info!(
                            "HPS:   [HTTP Proxy] {} ({}) @ {=[u8]:02X}",
                            name, addr_kind, entry.addr
                        );
                    } else {
                        info!(
                            "HPS:   [generic] {} ({}) @ {=[u8]:02X}",
                            name, addr_kind, entry.addr
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

            if session_active {
                warn!(
                    "HPS: HTTP Proxy service not observed during scan window; retrying after 2s..."
                );
            }
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
                    info!(
                        "HPS: Using {} address type; scanning until peer responds",
                        describe_addr_kind(target_kind)
                    );

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
                            window: Duration::from_millis(100),
                            timeout: Duration::from_secs(20),
                        },
                    };

                    info!("HPS: Awaiting connection (scan timeout 20s, overall 25s)");
                    let result =
                        with_timeout(Duration::from_secs(25), central.connect(&config)).await;

                    match result {
                        Ok(Ok(conn)) => {
                            info!("HPS: Link established; preparing GATT client session");

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

                            let mut request_rx = request_receiver();
                            let mut response_tx = response_sender();

                            info!("HPS: Managing GATT session and HTTP Proxy discovery");
                            let _ = join(
                                async {
                                    info!("HPS: ATT event pump active for connected peer");
                                    let result = gatt_client.task().await;
                                    warn!("HPS: GATT client task ended: {:?}", Debug2Format(&result));
                                    result
                                },
                                async {
                                    info!("HPS: Allowing peer to settle before discovery (2s grace period)...");
                                    Timer::after(Duration::from_secs(2)).await;

                                    info!("HPS: Discovering HTTP Proxy service (UUID 0x1823)...");
                                    let discovery = with_timeout(
                                        Duration::from_secs(15),
                                        discover_hps_service(&gatt_client)
                                    )
                                    .await;

                                    let hps_chars = match discovery {
                                        Ok(Ok(chars)) => {
                                            info!("HPS: HTTP Proxy characteristic map complete");
                                            chars
                                        }
                                        Ok(Err(e)) => {
                                            warn!("HPS: Service discovery failed: {:?}", e);
                                            drop(conn);
                                            return;
                                        }
                                        Err(_) => {
                                            warn!("HPS: Service discovery timeout after 15s");
                                            drop(conn);
                                            return;
                                        }
                                    };

                                    info!("HPS: HTTP Proxy link ready; notifying waiters");
                                    signal_ready();

                                    loop {
                                        match with_timeout(
                                            Duration::from_millis(250),
                                            request_rx.receive(),
                                        )
                                        .await
                                        {
                                            Ok(request) => {
                                                info!(
                                                    "HPS: Received {} request to {}",
                                                    request.method,
                                                    request.uri.as_str()
                                                );
                                                let response = execute_hps_request(
                                                    &gatt_client,
                                                    &hps_chars,
                                                    request,
                                                )
                                                .await;
                                                let _ = response_tx.send(response).await;
                                            }
                                            Err(_) => {
                                                if !conn.is_connected() {
                                                    info!("HPS: Link lost during idle wait");
                                                    break;
                                                }
                                            }
                                        }
                                    }
                                },
                            )
                            .await;

                            warn!("HPS: Peer disconnected; retrying after backoff");
                            Timer::after(Duration::from_secs(5)).await;
                            state = HpsServiceState::Disconnected;
                        }
                        Ok(Err(e)) => {
                            warn!("HPS: Connection failed: {:?}", Debug2Format(&e));
                            Timer::after(Duration::from_secs(5)).await;
                            state = HpsServiceState::Disconnected;
                        }
                        Err(_) => {
                            warn!("HPS: Connection timeout after 25s (scan timeout: 20s)");
                            Timer::after(Duration::from_secs(5)).await;
                            state = HpsServiceState::Disconnected;
                        }
                    }
                }
                HpsServiceState::Ready => {
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

    join(runner_task, service_task).await;
}
