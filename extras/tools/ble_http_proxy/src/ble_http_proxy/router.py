"""HTTP routing for the BLE proxy."""

from __future__ import annotations

import json
from dataclasses import dataclass
from typing import Optional
from urllib.parse import urljoin

import httpx

from .message_store import MessageStore
from .protocol import HttpRequest, HttpResponse, get_header, strip_hop_headers

_JSON_CONTENT_TYPE = "application/json"
_PROXY_HEADER = "X-Proxy-Target"
_TEXT_ASCII = "text/plain; charset=us-ascii"
_TEXT_UTF8 = "text/plain; charset=utf-8"


@dataclass
class ProxyError(Exception):
    status: int
    reason: str
    message: str


class HttpRouter:
    """Routes incoming HTTP requests to local or upstream handlers."""

    def __init__(self, store: MessageStore, default_upstream: Optional[str] = None) -> None:
        self._store = store
        if default_upstream and not default_upstream.startswith(("http://", "https://")):
            raise ValueError("default_upstream must be an absolute URL")
        if default_upstream and not default_upstream.endswith("/"):
            default_upstream = f"{default_upstream}/"
        self._default_upstream = default_upstream
        self._client: Optional[httpx.AsyncClient] = None

    async def __aenter__(self) -> "HttpRouter":
        self._client = httpx.AsyncClient()
        return self

    async def __aexit__(self, *_exc) -> None:
        if self._client is not None:
            await self._client.aclose()
            self._client = None

    async def handle(self, request: HttpRequest) -> HttpResponse:
        try:
            if request.path in {"/", "/health"}:
                return HttpResponse(
                    status=200,
                    reason="OK",
                    headers={"Content-Type": _TEXT_UTF8},
                    body=b"ok\n",
                )

            if request.path in {"/messages", "/get"} and request.method == "GET":
                payload = await self._store.render_snapshot()
                return HttpResponse(
                    status=200,
                    reason="OK",
                    headers={"Content-Type": _TEXT_ASCII},
                    body=payload,
                )

            if request.path in {"/messages", "/post"} and request.method == "POST":
                await self._handle_post_message(request)
                return HttpResponse(
                    status=204,
                    reason="No Content",
                    headers={},
                    body=b"",
                )

            target = self._resolve_target(request)
            if target:
                return await self._forward_upstream(request, target)

            raise ProxyError(404, "Not Found", "no route for request")
        except ProxyError as exc:
            return HttpResponse(
                status=exc.status,
                reason=exc.reason,
                headers={"Content-Type": _TEXT_UTF8},
                body=exc.message.encode("utf-8"),
            )

    async def _handle_post_message(self, request: HttpRequest) -> None:
        if request.body:
            try:
                content_type = get_header(request.headers, "Content-Type") or ""
                if content_type.startswith(_JSON_CONTENT_TYPE):
                    payload = json.loads(request.body.decode("utf-8"))
                    message = payload.get("message") or payload.get("content", "")
                else:
                    message = request.body.decode("utf-8")
            except (UnicodeDecodeError, json.JSONDecodeError) as exc:
                raise ProxyError(400, "Bad Request", "invalid JSON payload") from exc
        else:
            raise ProxyError(400, "Bad Request", "missing message body")

        if not message.strip():
            raise ProxyError(400, "Bad Request", "message cannot be empty")

        await self._store.append(message)

    def _resolve_target(self, request: HttpRequest) -> Optional[str]:
        if request.path.startswith(("http://", "https://")):
            return request.path

        header_value = get_header(request.headers, _PROXY_HEADER)
        if header_value:
            return header_value.strip()

        if self._default_upstream:
            relative_path = request.path.lstrip("/")
            return urljoin(self._default_upstream, relative_path)

        return None

    async def _forward_upstream(self, request: HttpRequest, target: str) -> HttpResponse:
        if self._client is None:
            raise ProxyError(500, "Server Error", "HTTP client not initialised")

        if not target.startswith(("http://", "https://")):
            raise ProxyError(400, "Bad Request", "Upstream target must be an absolute URL")

        headers = strip_hop_headers(request.headers)
        headers = {
            key: value
            for key, value in headers.items()
            if key.lower() not in {_PROXY_HEADER.lower(), "host", "content-length"}
        }

        try:
            resp = await self._client.request(
                request.method,
                target,
                headers=headers,
                content=request.body if request.body else None,
            )
        except httpx.HTTPError as exc:
            raise ProxyError(502, "Bad Gateway", str(exc)) from exc

        response_headers = strip_hop_headers(dict(resp.headers))
        return HttpResponse(
            status=resp.status_code,
            reason=resp.reason_phrase or "OK",
            headers=response_headers,
            body=resp.content,
        )
