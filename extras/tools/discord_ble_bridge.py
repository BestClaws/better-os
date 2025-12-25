#!/usr/bin/env python3
"""Discord-to-BLE HTTP bridge.

This utility connects to Discord using ``discord.py`` and mirrors the three
most recent messages into the BLE HTTP bridge service exposed by the firmware.
The embedded device periodically issues ``GET /get`` requests which are
answered with a plain-text payload containing the queued messages.
"""

import asyncio
import contextlib
import os
from collections import deque
from typing import Deque, Optional

import discord
from bleak import BleakClient, BleakScanner
from bleak.backends.device import BLEDevice

SERVICE_UUID = "408813df-3469-4f19-9d01-7c87f7f04001"
REQUEST_CHARACTERISTIC_UUID = "408813df-3469-4f19-9d01-7c87f7f04002"
RESPONSE_CHARACTERISTIC_UUID = "408813df-3469-4f19-9d01-7c87f7f04003"
DEVICE_NAME = "Better HTTP"
REQUEST_TERMINATOR = b"\r\n\r\n"
POLL_PATH = "/get"
QUEUE_SIZE = 3
MAX_MESSAGE_LEN = 64

HTTP_OK = (200, "OK")
HTTP_NOT_FOUND = (404, "Not Found")


def sanitize_ascii(text: str) -> str:
    filtered = [ch for ch in text if 32 <= ord(ch) < 127]
    trimmed = "".join(filtered)
    return (trimmed[:MAX_MESSAGE_LEN] or "...").strip()


async def discover_device() -> Optional[BLEDevice]:
    def _filter(device, advertisement_data):
        if device.name == DEVICE_NAME:
            return True
        if advertisement_data and advertisement_data.service_uuids:
            return SERVICE_UUID.lower() in {
                uuid.lower() for uuid in advertisement_data.service_uuids
            }
        return False

    return await BleakScanner.find_device_by_filter(_filter, timeout=10.0)


def build_http_response(status: int, reason: str, body: str) -> bytes:
    body_bytes = body.encode("ascii", errors="ignore")
    headers = [
        f"HTTP/1.1 {status} {reason}",
        "Content-Type: text/plain; charset=us-ascii",
        f"Content-Length: {len(body_bytes)}",
        "Connection: close",
        "",
        "",
    ]
    header_bytes = "\r\n".join(headers).encode("ascii")
    return header_bytes + body_bytes


def parse_path(request_bytes: bytes) -> str:
    try:
        text = request_bytes.decode("ascii", errors="ignore")
    except UnicodeDecodeError:
        return "/"
    first_line = text.split("\r\n", 1)[0]
    parts = first_line.split()
    if len(parts) >= 2:
        return parts[1]
    return "/"


async def run_ble_bridge(
    request_queue: "asyncio.Queue[bytes]",
    messages: Deque[str],
    lock: asyncio.Lock,
) -> None:
    while True:
        device = await discover_device()
        if device is None:
            print("BLE device not found, retrying in 5s...")
            await asyncio.sleep(5)
            continue

        print(f"Connecting to {device.address} ({DEVICE_NAME})")
        try:
            async with BleakClient(device) as client:
                await client.start_notify(
                    REQUEST_CHARACTERISTIC_UUID,
                    lambda _h, data: request_queue.put_nowait(bytes(data)),
                )
                print("Notifications enabled; awaiting requests...")
                buffer = bytearray()

                while True:
                    chunk = await request_queue.get()
                    buffer.extend(chunk)
                    if REQUEST_TERMINATOR not in buffer:
                        continue

                    request_payload = bytes(buffer)
                    buffer.clear()

                    path = parse_path(request_payload)
                    if path != POLL_PATH:
                        response = build_http_response(*HTTP_NOT_FOUND, "not found")
                    else:
                        async with lock:
                            body = "\n".join(messages)
                        response = build_http_response(*HTTP_OK, body)

                    await client.write_gatt_char(
                        RESPONSE_CHARACTERISTIC_UUID,
                        response,
                        response=True,
                    )
        except asyncio.CancelledError:  # pragma: no cover - graceful shutdown
            raise
        except Exception as err:  # pragma: no cover - best-effort recovery
            print(f"BLE bridge error: {err}")
            await asyncio.sleep(5)


async def main() -> None:
    token = os.environ.get("DISCORD_TOKEN")
    if not token:
        raise RuntimeError("DISCORD_TOKEN environment variable must be set")

    intents = discord.Intents.default()
    intents.message_content = True

    messages: Deque[str] = deque(["...", "...", "..."], maxlen=QUEUE_SIZE)
    request_queue: "asyncio.Queue[bytes]" = asyncio.Queue()
    queue_lock = asyncio.Lock()

    client = discord.Client(intents=intents)

    @client.event
    async def on_ready() -> None:
        print(f"Discord connected as {client.user}")

    @client.event
    async def on_message(message: discord.Message) -> None:
        if message.author == client.user:
            return
        sanitized = sanitize_ascii(f"{message.author.display_name}: {message.content}")
        async with queue_lock:
            messages.append(sanitized)

    ble_task = asyncio.create_task(run_ble_bridge(request_queue, messages, queue_lock))

    try:
        await client.start(token)
    finally:
        ble_task.cancel()
        with contextlib.suppress(asyncio.CancelledError):
            await ble_task


if __name__ == "__main__":
    try:
        asyncio.run(main())
    except KeyboardInterrupt:
        print("Interrupted; shutting down")
