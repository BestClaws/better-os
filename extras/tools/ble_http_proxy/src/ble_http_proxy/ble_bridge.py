"""BLE transport for the HTTP proxy."""

from __future__ import annotations

import asyncio
import logging
from dataclasses import dataclass
from typing import Awaitable, Callable, Optional

from bleak import BleakClient, BleakError, BleakScanner

from .protocol import (
    HttpProtocolError,
    HttpRequest,
    HttpResponse,
    build_http_response,
    parse_http_request,
)

_REQUEST_UUID = "408813df-3469-4f19-9d01-7c87f7f04002"
_RESPONSE_UUID = "408813df-3469-4f19-9d01-7c87f7f04003"
_DEFAULT_DEVICE_NAME = "Better HTTP"
_WRITE_CHUNK = 120


class RequestAssembler:
    """Incrementally assembles HTTP requests from BLE notifications."""

    def __init__(self) -> None:
        self._buffer = bytearray()
        self._expected_length: Optional[int] = None
        self._header_end: Optional[int] = None

    def feed(self, chunk: bytes) -> Optional[bytes]:
        self._buffer.extend(chunk)

        if self._header_end is None:
            idx = self._buffer.find(b"\r\n\r\n")
            if idx != -1:
                self._header_end = idx + 4
                header = bytes(self._buffer[: self._header_end])
                content_length = self._extract_content_length(header)
                self._expected_length = self._header_end + content_length

        if self._expected_length is not None and len(self._buffer) >= self._expected_length:
            request = bytes(self._buffer[: self._expected_length])
            self._buffer.clear()
            self._header_end = None
            self._expected_length = None
            return request
        return None

    @staticmethod
    def _extract_content_length(header: bytes) -> int:
        try:
            text = header.decode("iso-8859-1")
        except UnicodeDecodeError:
            return 0
        for line in text.split("\r\n"):
            if not line or ":" not in line:
                continue
            name, value = line.split(":", 1)
            if name.strip().lower() == "content-length":
                try:
                    return int(value.strip())
                except ValueError:
                    return 0
        return 0


@dataclass
class BridgeConfig:
    device_name: str = _DEFAULT_DEVICE_NAME
    service_uuid: Optional[str] = "408813df-3469-4f19-9d01-7c87f7f04001"


class BleHttpBridge:
    """Maintains a BLE link and forwards requests to the supplied handler."""

    def __init__(
        self,
        config: BridgeConfig,
        handler: Callable[[HttpRequest], Awaitable[HttpResponse]],
    ) -> None:
        self._config = config
        self._handler = handler
        self._assembler = RequestAssembler()
        self._queue: asyncio.Queue[bytes] = asyncio.Queue(maxsize=2)
        self._loop: Optional[asyncio.AbstractEventLoop] = None

    async def run(self) -> None:
        self._loop = asyncio.get_running_loop()
        while True:
            device = await self._discover()
            if device is None:
                logging.info("BLE device not found, retrying in 5s")
                await asyncio.sleep(5)
                continue

            try:
                async with BleakClient(device) as client:
                    await client.start_notify(_REQUEST_UUID, self._on_notify)
                    await self._pump_requests(client)
            except asyncio.CancelledError:
                raise
            except BleakError as exc:
                logging.warning("BLE error: %s", exc)
                await asyncio.sleep(5)
            except Exception as exc:  # pragma: no cover - defensive path
                logging.exception("Unhandled proxy error: %s", exc)
                await asyncio.sleep(5)

    async def _pump_requests(self, client: BleakClient) -> None:
        while True:
            payload = await self._queue.get()
            try:
                request = parse_http_request(payload)
                response = await self._handler(request)
            except HttpProtocolError as exc:
                response = HttpResponse(
                    status=400,
                    reason="Bad Request",
                    headers={"Content-Type": "text/plain; charset=utf-8"},
                    body=str(exc).encode("utf-8"),
                )
            except Exception as exc:  # pragma: no cover - defensive logging path
                response = HttpResponse(
                    status=500,
                    reason="Server Error",
                    headers={"Content-Type": "text/plain; charset=utf-8"},
                    body=str(exc).encode("utf-8"),
                )
            raw = build_http_response(response)
            for offset in range(0, len(raw), _WRITE_CHUNK):
                chunk = raw[offset : offset + _WRITE_CHUNK]
                await client.write_gatt_char(_RESPONSE_UUID, chunk, response=True)

    async def _discover(self):
        def _match(device, advertisement_data):
            if device.name == self._config.device_name:
                return True
            if (
                self._config.service_uuid
                and advertisement_data
                and advertisement_data.service_uuids
            ):
                needle = self._config.service_uuid.lower()
                return any(uuid.lower() == needle for uuid in advertisement_data.service_uuids)
            return False

        return await BleakScanner.find_device_by_filter(_match, timeout=10.0)

    def _on_notify(self, _handle: int, data: bytearray) -> None:
        request = self._assembler.feed(bytes(data))
        if request is not None:
            if self._loop is None:
                return

            def _enqueue() -> None:
                try:
                    self._queue.put_nowait(request)
                except asyncio.QueueFull:
                    try:
                        _ = self._queue.get_nowait()
                    except asyncio.QueueEmpty:
                        pass
                    self._queue.put_nowait(request)

            self._loop.call_soon_threadsafe(_enqueue)
