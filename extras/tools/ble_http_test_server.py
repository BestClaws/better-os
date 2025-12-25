#!/usr/bin/env python3
"""Minimal BLE-backed HTTP test server.

This script uses the `bleak` library to connect to the firmware's
HTTP bridge service and emulates a tiny HTTP server. Whenever a full
request is received (detected via ``\r\n\r\n``), a canned HTTP
response is written back over the response characteristic.

Usage::

    pip install bleak
    python extras/tools/ble_http_test_server.py

The script scans for a peripheral advertising the BLE HTTP Bridge
service UUID or the default device name ("Better HTTP"). Adjust the
constants below if you change the firmware configuration.
"""

import asyncio
from typing import Optional

from bleak import BleakClient, BleakScanner

SERVICE_UUID = "408813df-3469-4f19-9d01-7c87f7f04001"
REQUEST_CHARACTERISTIC_UUID = "408813df-3469-4f19-9d01-7c87f7f04002"
RESPONSE_CHARACTERISTIC_UUID = "408813df-3469-4f19-9d01-7c87f7f04003"
DEVICE_NAME = "Better HTTP"

HTTP_RESPONSE = (
    b"HTTP/1.1 200 OK\r\n"
    b"Content-Type: text/plain; charset=utf-8\r\n"
    b"Content-Length: 13\r\n"
    b"Connection: close\r\n\r\n"
    b"hello, world!"
)

REQUEST_TERMINATOR = b"\r\n\r\n"
SCAN_TIMEOUT = 15.0


async def discover_device() -> Optional[str]:
    def _filter(device, advertisement_data):
        if device.name == DEVICE_NAME:
            return True
        if advertisement_data and advertisement_data.service_uuids:
            return SERVICE_UUID.lower() in {
                uuid.lower() for uuid in advertisement_data.service_uuids
            }
        return False

    device = await BleakScanner.find_device_by_filter(_filter, timeout=SCAN_TIMEOUT)
    if device is None:
        return None
    return device.address


async def run_server() -> None:
    address = await discover_device()
    if address is None:
        raise RuntimeError(
            "Unable to find BLE HTTP bridge. Ensure the firmware is advertising and retry."
        )

    print(f"Connecting to {address} ({DEVICE_NAME})...")

    queue: asyncio.Queue[bytes] = asyncio.Queue()

    def handle_request(_: int, data: bytearray) -> None:
        queue.put_nowait(bytes(data))

    async with BleakClient(address) as client:
        if hasattr(client, "exchange_mtu"):
            try:
                mtu = await client.exchange_mtu(247)
                print(f"Negotiated MTU: {mtu}")
            except Exception as err:  # pragma: no cover - MTU exchange best-effort
                print(f"MTU exchange failed ({err}); continuing with default MTU")
        else:
            mtu = getattr(client, "mtu_size", None)
            if mtu:
                print(f"Using existing MTU: {mtu}")

        await client.start_notify(REQUEST_CHARACTERISTIC_UUID, handle_request)
        print("Notifications enabled on request characteristic.")
        print("Connected. Waiting for HTTP requests over BLE...")
        buffer = bytearray()

        while True:
            chunk = await queue.get()
            buffer.extend(chunk)
            print(f"Received {len(chunk)} bytes: {chunk!r}")

            if REQUEST_TERMINATOR in buffer:
                request_text = buffer.decode(errors="replace")
                print("\n--- HTTP Request ---")
                print(request_text)
                print("--------------------\n")

                await client.write_gatt_char(
                    RESPONSE_CHARACTERISTIC_UUID,
                    HTTP_RESPONSE,
                    response=True,
                )
                print("Response sent (13 bytes).\n")
                buffer.clear()


async def main() -> None:
    try:
        await run_server()
    except KeyboardInterrupt:
        print("Interrupted, exiting")


if __name__ == "__main__":
    asyncio.run(main())
