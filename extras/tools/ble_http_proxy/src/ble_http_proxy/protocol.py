"""HTTP parsing and serialization helpers for the BLE proxy."""

from __future__ import annotations

from dataclasses import dataclass
from typing import Dict, Iterable, Tuple

_HEADER_SEPARATOR = b"\r\n\r\n"
_HOP_BY_HOP_HEADERS = {
    "connection",
    "keep-alive",
    "proxy-authenticate",
    "proxy-authorization",
    "te",
    "trailers",
    "transfer-encoding",
    "upgrade",
}


@dataclass
class HttpRequest:
    method: str
    path: str
    version: str
    headers: Dict[str, str]
    body: bytes


@dataclass
class HttpResponse:
    status: int
    reason: str
    headers: Dict[str, str]
    body: bytes


class HttpProtocolError(RuntimeError):
    """Raised when an incoming request cannot be parsed."""


def split_request(buffer: bytes) -> Tuple[bytes, bytes]:
    """Split raw request bytes into (head, body) at the header terminator."""

    marker = buffer.find(_HEADER_SEPARATOR)
    if marker == -1:
        raise HttpProtocolError("incomplete header")
    head = buffer[:marker]
    body = buffer[marker + len(_HEADER_SEPARATOR) :]
    return head, body


def parse_http_request(payload: bytes) -> HttpRequest:
    """Parse an HTTP/1.1 request from raw bytes.

    The request is expected to include a `Content-Length` header whenever a body is
    present. The body may contain arbitrary binary data and is returned verbatim.
    """

    head, body = split_request(payload)
    try:
        lines = head.decode("iso-8859-1").split("\r\n")
    except UnicodeDecodeError as exc:
        raise HttpProtocolError("header is not ISO-8859-1") from exc

    if not lines or not lines[0]:
        raise HttpProtocolError("missing request line")

    parts = lines[0].split()
    if len(parts) != 3:
        raise HttpProtocolError("malformed request line")
    method, path, version = parts

    headers: Dict[str, str] = {}
    for line in lines[1:]:
        if not line:
            continue
        if ":" not in line:
            raise HttpProtocolError(f"malformed header: {line!r}")
        name, value = line.split(":", 1)
        headers[name.strip()] = value.lstrip()

    content_length = int(headers.get("Content-Length", "0"))
    if len(body) < content_length:
        raise HttpProtocolError("body truncated")

    return HttpRequest(
        method=method.upper(),
        path=path,
        version=version,
        headers={key: value for key, value in headers.items()},
        body=body[:content_length],
    )


def build_http_response(response: HttpResponse) -> bytes:
    """Serialize an :class:`HttpResponse` into raw HTTP/1.1 bytes."""

    status_line = f"HTTP/1.1 {response.status} {response.reason}".encode("ascii", "ignore")

    headers = dict(response.headers)
    headers["Content-Length"] = str(len(response.body))
    headers.setdefault("Connection", "close")

    header_bytes = [status_line]
    for name, value in headers.items():
        header_bytes.append(f"{name}: {value}".encode("ascii", "ignore"))
    header_bytes.append(b"")
    header_bytes.append(b"")

    return b"\r\n".join(header_bytes) + response.body


def strip_hop_headers(headers: Dict[str, str]) -> Dict[str, str]:
    """Return a copy of *headers* without hop-by-hop entries."""

    return {k: v for k, v in headers.items() if k.lower() not in _HOP_BY_HOP_HEADERS}


def get_header(headers: Dict[str, str], name: str) -> str | None:
    """Case-insensitive header lookup."""

    lower = name.lower()
    for key, value in headers.items():
        if key.lower() == lower:
            return value
    return None
