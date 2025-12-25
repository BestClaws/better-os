use defmt::{debug, info, warn, Debug2Format};
use embassy_futures::select::{select, Either};
use trouble_host::prelude::*;

use super::{
    deliver_error, deliver_response, HttpBridgeError, HttpRequestBuffer, HttpRequestReceiver,
    HttpResponseBuffer, MAX_HTTP_RESPONSE,
};

use crate::libs::bluetooth::{
    event_sender, BlePacket, BluetoothEvent, EventSender, GattServiceStatus, BLE_PACKET_CAPACITY,
};

/// BLE advertised device name.
pub const HTTP_DEVICE_NAME: &str = "Better HTTP";
/// Display name for defmt logging.
pub const HTTP_SERVICE_NAME: &str = "BLE HTTP Bridge";
/// Request characteristic label for logs.
pub const HTTP_REQUEST_CHAR_NAME: &str = "HTTP Request Channel";
/// Response characteristic label for logs.
pub const HTTP_RESPONSE_CHAR_NAME: &str = "HTTP Response Channel";
/// Maximum number of bytes buffered while streaming responses.
const RESPONSE_BUFFER_CAPACITY: usize = MAX_HTTP_RESPONSE;
/// Notify chunk size (20 bytes for legacy MTU compatibility).
pub const HTTP_NOTIFY_CHUNK: usize = 20;

/// BLE HTTP bridge service definition (vendor specific UUIDs).
#[gatt_server]
pub struct HttpBridgeServer {
    pub http: HttpBridgeService,
}

/// GATT service implementing the `BLE HTTP Bridge`.
#[gatt_service(uuid = "408813df-3469-4f19-9d01-7c87f7f04001")]
pub struct HttpBridgeService {
    /// `HTTP Request Channel` characteristic.
    #[characteristic(
        uuid = "408813df-3469-4f19-9d01-7c87f7f04002",
        read,
        notify,
        value = BlePacket::new(),
    )]
    pub request: BlePacket,
    /// `HTTP Response Channel` characteristic.
    #[characteristic(
        uuid = "408813df-3469-4f19-9d01-7c87f7f04003",
        read,
        write,
        notify,
        value = BlePacket::new(),
    )]
    pub response: BlePacket,
}

/// Run a complete HTTP client session on an active GATT connection.
pub async fn http_client_session(
    request_char: &Characteristic<BlePacket>,
    response_char: &Characteristic<BlePacket>,
    conn: &GattConnection<'_, '_, DefaultPacketPool>,
    status_events: EventSender,
    tx_events: EventSender,
    rx_events: EventSender,
    request_rx: &mut HttpRequestReceiver,
) -> Result<(), Error> {
    let mut response_buffer = HttpResponseBuffer::with_capacity(RESPONSE_BUFFER_CAPACITY);
    let mut reported_complete = false;
    let mut notifications_enabled = request_char.cccd_handle.map(|_| false).unwrap_or(true);
    let mut request_in_flight = false;
    let mut pending_request: Option<HttpRequestBuffer> = None;

    loop {
        if notifications_enabled && !request_in_flight && pending_request.is_none() {
            match select(request_rx.receive(), conn.next()).await {
                Either::First(request) => {
                    pending_request = Some(request);
                    response_buffer.clear();
                    reported_complete = false;
                }
                Either::Second(event) => {
                    if handle_connection_event(
                        event,
                        request_char,
                        response_char,
                        &status_events,
                        &rx_events,
                        &mut response_buffer,
                        &mut reported_complete,
                        &mut request_in_flight,
                        &mut notifications_enabled,
                    )
                    .await?
                    {
                        break;
                    }
                    continue;
                }
            }
        } else {
            let event = conn.next().await;
            if handle_connection_event(
                event,
                request_char,
                response_char,
                &status_events,
                &rx_events,
                &mut response_buffer,
                &mut reported_complete,
                &mut request_in_flight,
                &mut notifications_enabled,
            )
            .await?
            {
                break;
            }
            continue;
        }

        if let Some(request) = pending_request.take() {
            let _ = status_events.try_send(BluetoothEvent::GattServiceStatus(
                GattServiceStatus::SendingRequest,
            ));
            if let Err(err) =
                send_http_request(request_char, conn, &tx_events, request.as_slice()).await
            {
                warn!(
                    "[gatt] failed to send HTTP request: {:?}",
                    Debug2Format(&err)
                );
                deliver_error(HttpBridgeError::Internal);
                return Err(err);
            }
            let _ = status_events.try_send(BluetoothEvent::GattServiceStatus(
                GattServiceStatus::AwaitingResponse,
            ));
            request_in_flight = true;
        }
    }

    if pending_request.is_some() {
        deliver_error(HttpBridgeError::Disconnected);
    }

    Ok(())
}

#[allow(clippy::too_many_arguments)]
async fn handle_connection_event(
    event: GattConnectionEvent<'_, '_, DefaultPacketPool>,
    request_char: &Characteristic<BlePacket>,
    response_char: &Characteristic<BlePacket>,
    status_events: &EventSender,
    rx_events: &EventSender,
    response_buffer: &mut HttpResponseBuffer,
    reported_complete: &mut bool,
    request_in_flight: &mut bool,
    notifications_enabled: &mut bool,
) -> Result<bool, Error> {
    match event {
        GattConnectionEvent::Disconnected { reason } => {
            info!("[gatt] disconnected: {:?}", reason);
            if *request_in_flight {
                deliver_error(HttpBridgeError::Disconnected);
            }
            if !response_buffer.is_empty() && !*reported_complete {
                let _ = status_events.try_send(BluetoothEvent::GattServiceStatus(
                    GattServiceStatus::ResponseComplete,
                ));
            }
            *request_in_flight = false;
            response_buffer.clear();
            *reported_complete = false;
            return Ok(true);
        }
        GattConnectionEvent::Gatt { event } => {
            if let GattEvent::Write(write) = &event {
                if Some(write.handle()) == request_char.cccd_handle {
                    let enabled = write
                        .data()
                        .first()
                        .map(|flags| flags & 0x01 != 0)
                        .unwrap_or(false);
                    *notifications_enabled = enabled;

                    debug!(
                        "[gatt] {} notifications {}",
                        HTTP_REQUEST_CHAR_NAME,
                        if *notifications_enabled {
                            "enabled"
                        } else {
                            "disabled"
                        }
                    );

                    if !*notifications_enabled {
                        *request_in_flight = false;
                        response_buffer.clear();
                        *reported_complete = false;
                        let _ = status_events.try_send(BluetoothEvent::GattServiceStatus(
                            GattServiceStatus::Connected,
                        ));
                    }
                } else if write.handle() == response_char.handle {
                    debug!(
                        "[gatt] received {} chunk ({} bytes)",
                        HTTP_RESPONSE_CHAR_NAME,
                        write.data().len()
                    );

                    let available = RESPONSE_BUFFER_CAPACITY.saturating_sub(response_buffer.len());
                    if available > 0 {
                        let take_len = available.min(write.data().len());
                        response_buffer.extend_from_slice(&write.data()[..take_len]);
                        if take_len < write.data().len() {
                            warn!("[gatt] response chunk truncated (buffer full)");
                        }
                    } else {
                        warn!("[gatt] response chunk dropped (buffer full)");
                    }

                    match BlePacket::from_slice(write.data()) {
                        Ok(packet) => {
                            let _ = rx_events.try_send(BluetoothEvent::GattValueReceived(packet));
                        }
                        Err(_) => {
                            let mut packet = BlePacket::new();
                            let copy_len = write.data().len().min(BLE_PACKET_CAPACITY);
                            packet.extend_from_slice(&write.data()[..copy_len]).ok();
                            let _ = rx_events.try_send(BluetoothEvent::GattValueReceived(packet));
                        }
                    }

                    if !*reported_complete {
                        let copy_len = response_buffer.len().min(RESPONSE_BUFFER_CAPACITY);
                        let mut delivered = HttpResponseBuffer::with_capacity(copy_len);
                        delivered.extend_from_slice(&response_buffer[..copy_len]);
                        if response_buffer.len() > RESPONSE_BUFFER_CAPACITY {
                            warn!(
                                "[gatt] delivered response truncated ({} / {})",
                                copy_len,
                                response_buffer.len()
                            );
                        }
                        deliver_response(delivered);
                        let _ = status_events.try_send(BluetoothEvent::GattServiceStatus(
                            GattServiceStatus::ResponseComplete,
                        ));
                        *reported_complete = true;
                        *request_in_flight = false;
                    }
                }
            }

            if let Ok(reply) = event.accept() {
                reply.send().await;
            }
        }
        _ => {}
    }

    Ok(false)
}

async fn send_http_request(
    request_char: &Characteristic<BlePacket>,
    conn: &GattConnection<'_, '_, DefaultPacketPool>,
    events: &EventSender,
    payload: &[u8],
) -> Result<(), Error> {
    let mut chunk_index = 0u32;
    for chunk in payload.chunks(HTTP_NOTIFY_CHUNK) {
        let mut packet = BlePacket::new();
        if packet.extend_from_slice(chunk).is_err() {
            warn!("[gatt] request chunk truncated");
            packet.extend_from_slice(&chunk[..BLE_PACKET_CAPACITY]).ok();
        }

        debug!(
            "[gatt] sending {} chunk #{} ({} bytes)",
            HTTP_REQUEST_CHAR_NAME,
            chunk_index,
            packet.len()
        );
        let event_packet = packet.clone();
        request_char.notify(conn, &packet).await?;
        let _ = events.try_send(BluetoothEvent::GattValueSent(event_packet));
        chunk_index = chunk_index.saturating_add(1);
    }

    Ok(())
}

pub async fn advertise_http<'values, 'server, C>(
    peripheral: &mut Peripheral<'values, C, DefaultPacketPool>,
    server: &'server HttpBridgeServer<'values>,
) -> Result<GattConnection<'values, 'server, DefaultPacketPool>, BleHostError<C::Error>>
where
    C: Controller,
{
    let mut adv_data = [0u8; 31];
    let len = AdStructure::encode_slice(
        &[
            AdStructure::Flags(LE_GENERAL_DISCOVERABLE | BR_EDR_NOT_SUPPORTED),
            AdStructure::CompleteLocalName(HTTP_DEVICE_NAME.as_bytes()),
        ],
        &mut adv_data,
    )?;

    let advertiser = peripheral
        .advertise(
            &Default::default(),
            Advertisement::ConnectableScannableUndirected {
                adv_data: &adv_data[..len],
                scan_data: &[],
            },
        )
        .await?;

    let conn = advertiser.accept().await?.with_attribute_server(server)?;
    info!("[gatt] {} connection established", HTTP_SERVICE_NAME);
    Ok(conn)
}
