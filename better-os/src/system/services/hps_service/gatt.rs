use crate::libs::hps::error::HpsError;
use crate::libs::hps::types::{
    DataStatus, HpsUuids, HttpResponse, MAX_BODY_SIZE, MAX_HEADERS_SIZE,
};
use alloc::string::String;
use alloc::vec::Vec;
use defmt::{info, warn, Debug2Format};
use embassy_time::{with_timeout, Duration, Instant, Timer};
use trouble_host::prelude::*;
use trouble_host::types::gatt_traits::{AsGatt, FromGatt, FromGattError};

use super::types::{HpsRequest, HpsResponse};

pub(crate) struct HpsGattProfile {
    pub uri: Characteristic<GattBuffer<512>>,
    pub headers: Characteristic<GattBuffer<512>>,
    pub status_code: Characteristic<GattBuffer<3>>,
    pub entity_body: Characteristic<GattBuffer<512>>,
    pub control_point: Characteristic<u8>,
    pub https_security: Characteristic<u8>,
}

#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct GattBuffer<const N: usize>;

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

pub(crate) const STATUS_POLL_INTERVAL_MS: u64 = 120;
pub(crate) const STATUS_WAIT_TIMEOUT_MS: u64 = 5_000;
pub(crate) const SLOW_STAGE_LOG_THRESHOLD_MS: u64 = 250;

pub(crate) async fn execute_hps_request<C: Controller, P: PacketPool>(
    gatt: &GattClient<'_, C, P, 10>,
    profile: &HpsGattProfile,
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
    if let Err(e) = gatt.write_characteristic(&profile.uri, uri_bytes).await {
        warn!("HPS: Failed to write URI: {:?}", Debug2Format(&e));
        return Err(HpsError::BleError);
    }
    log_stage_timing("write_uri", stage_start.elapsed().as_millis());

    if !headers_bytes.is_empty() {
        let stage_start = Instant::now();
        if let Err(e) = gatt
            .write_characteristic(&profile.headers, headers_bytes)
            .await
        {
            warn!("HPS: Failed to write headers: {:?}", Debug2Format(&e));
            return Err(HpsError::BleError);
        }
        log_stage_timing("write_headers", stage_start.elapsed().as_millis());
    } else {
        let stage_start = Instant::now();
        if let Err(e) = gatt.write_characteristic(&profile.headers, &[]).await {
            warn!("HPS: Failed to write empty headers: {:?}", Debug2Format(&e));
            return Err(HpsError::BleError);
        }
        log_stage_timing("write_headers_empty", stage_start.elapsed().as_millis());
    }

    if !body_slice.is_empty() {
        let stage_start = Instant::now();
        if let Err(e) = gatt
            .write_characteristic(&profile.entity_body, body_slice)
            .await
        {
            warn!("HPS: Failed to write body: {:?}", Debug2Format(&e));
            return Err(HpsError::BleError);
        }
        log_stage_timing("write_body", stage_start.elapsed().as_millis());
    } else {
        let stage_start = Instant::now();
        if let Err(e) = gatt.write_characteristic(&profile.entity_body, &[]).await {
            warn!("HPS: Failed to write empty body: {:?}", Debug2Format(&e));
            return Err(HpsError::BleError);
        }
        log_stage_timing("write_body_empty", stage_start.elapsed().as_millis());
    }

    let stage_start = Instant::now();
    let method_opcode = method as u8;
    if let Err(e) = gatt
        .write_characteristic(&profile.control_point, &[method_opcode])
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
            match gatt
                .read_characteristic(&profile.status_code, &mut status_buf)
                .await
            {
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
            .read_characteristic(&profile.headers, &mut headers_buf)
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
        match gatt
            .read_characteristic(&profile.entity_body, &mut body_buf)
            .await
        {
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

pub(crate) async fn discover_hps_service<
    T: Controller,
    P: PacketPool,
    const MAX_SERVICES: usize,
>(
    gatt: &GattClient<'_, T, P, MAX_SERVICES>,
) -> Result<HpsGattProfile, HpsError> {
    info!("HPS: Discovering service and characteristics");

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

    let uri = match gatt
        .characteristic_by_uuid::<GattBuffer<512>>(
            &service,
            &Uuid::Uuid16(HpsUuids::URI.to_le_bytes()),
        )
        .await
    {
        Ok(characteristic) => {
            info!(
                "HPS: Found URI characteristic (handle={})",
                characteristic.handle
            );
            characteristic
        }
        Err(_) => {
            warn!("HPS: URI characteristic not found (mandatory)");
            return Err(HpsError::NotFound);
        }
    };

    let headers = match gatt
        .characteristic_by_uuid::<GattBuffer<512>>(
            &service,
            &Uuid::Uuid16(HpsUuids::HTTP_HEADERS.to_le_bytes()),
        )
        .await
    {
        Ok(characteristic) => {
            info!(
                "HPS: Found HTTP Headers characteristic (handle={})",
                characteristic.handle
            );
            characteristic
        }
        Err(_) => {
            warn!("HPS: HTTP Headers characteristic not found (mandatory)");
            return Err(HpsError::NotFound);
        }
    };

    let status_code = match gatt
        .characteristic_by_uuid::<GattBuffer<3>>(
            &service,
            &Uuid::Uuid16(HpsUuids::HTTP_STATUS_CODE.to_le_bytes()),
        )
        .await
    {
        Ok(characteristic) => {
            info!(
                "HPS: Found HTTP Status Code characteristic (handle={}, CCCD={:?})",
                characteristic.handle, characteristic.cccd_handle
            );
            characteristic
        }
        Err(_) => {
            warn!("HPS: HTTP Status Code characteristic not found (mandatory)");
            return Err(HpsError::NotFound);
        }
    };

    let entity_body = match gatt
        .characteristic_by_uuid::<GattBuffer<512>>(
            &service,
            &Uuid::Uuid16(HpsUuids::HTTP_ENTITY_BODY.to_le_bytes()),
        )
        .await
    {
        Ok(characteristic) => {
            info!(
                "HPS: Found HTTP Entity Body characteristic (handle={})",
                characteristic.handle
            );
            characteristic
        }
        Err(_) => {
            warn!("HPS: HTTP Entity Body characteristic not found (mandatory)");
            return Err(HpsError::NotFound);
        }
    };

    let control_point = match gatt
        .characteristic_by_uuid::<u8>(
            &service,
            &Uuid::Uuid16(HpsUuids::HTTP_CONTROL_POINT.to_le_bytes()),
        )
        .await
    {
        Ok(characteristic) => {
            info!(
                "HPS: Found HTTP Control Point characteristic (handle={})",
                characteristic.handle
            );
            characteristic
        }
        Err(_) => {
            warn!("HPS: HTTP Control Point characteristic not found (mandatory)");
            return Err(HpsError::NotFound);
        }
    };

    let https_security = match gatt
        .characteristic_by_uuid::<u8>(
            &service,
            &Uuid::Uuid16(HpsUuids::HTTPS_SECURITY.to_le_bytes()),
        )
        .await
    {
        Ok(characteristic) => {
            info!(
                "HPS: Found HTTPS Security characteristic (handle={})",
                characteristic.handle
            );
            characteristic
        }
        Err(_) => {
            warn!("HPS: HTTPS Security characteristic not found (mandatory)");
            return Err(HpsError::NotFound);
        }
    };

    info!("HPS: Successfully discovered all HPS characteristics");
    Ok(HpsGattProfile {
        uri,
        headers,
        status_code,
        entity_body,
        control_point,
        https_security,
    })
}
