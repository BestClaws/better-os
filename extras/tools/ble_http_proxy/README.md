# BLE HTTP Proxy

`ble-http-proxy` is a lightweight companion application for Better OS devices. It listens for
HTTP requests forwarded over the firmware's `BLE HTTP Bridge` service and answers them by either:

- Serving a small local message board that the firmware can poll, or
- Forwarding the request to an arbitrary upstream HTTP(S) endpoint selected by the device.

The goal is to provide a generic reference implementation that can be adapted to other
platforms (desktop, Android, iOS) without any third-party service dependencies.

## Features

- Discovers and connects to the watch's BLE HTTP bridge using Bleak
- Parses HTTP/1.1 requests and streams responses back over BLE notifications
- Exposes a `/messages` (and legacy `/get`) endpoint backed by a ring buffer of ASCII lines
- Accepts `POST /messages` with either JSON or plain text payloads to update the buffer
- Proxies any request that targets an absolute URL or supplies an `X-Proxy-Target` header
- Optional `--default-upstream` flag to resolve relative paths against a base URL

## Installation

```bash
python -m venv .venv
source .venv/bin/activate
pip install -e extras/tools/ble_http_proxy
```

## Usage

```bash
ble-http-proxy --device-name "Better HTTP"
```

### Command-line options

```
ble-http-proxy [--device-name "Better HTTP"] \
          [--service-uuid 408813df-3469-4f19-9d01-7c87f7f04001] \
          [--default-upstream https://example.com/api] \
          [--seed "First message"]
```

- `--device-name`: BLE advertised name to match (default: `Better HTTP`)
- `--service-uuid`: Service UUID to filter during discovery
- `--default-upstream`: Optional base URL used when the firmware sends relative paths
- `--seed`: Seed message (may be supplied multiple times) for the local message board

## Request Flow

1. The device enqueues an HTTP request into the BLE bridge (see `http_bridge` Rust module).
2. The proxy receives the bytes, parses headers and body, and produces a structured request.
3. Depending on the path/headers:
  - `/messages` or `/get` are served by the local message board.
  - `/messages` `POST` updates the message board.
  - Requests that include absolute URLs, `X-Proxy-Target`, or can be resolved with
    `--default-upstream` are proxied upstream via `httpx`.
4. The proxy builds an HTTP/1.1 response (status line, headers, binary body) and streams it back to
  the device over the BLE response characteristic.

## Android Porting Notes

- Replace Bleak with Android's `BluetoothGatt` APIs to subscribe to the same service/characteristics.
- Reuse the `RequestAssembler` and HTTP parsing helpers; the protocol is byte-compatible.
- Swap out the asyncio plumbing for the platform's preferred networking stack.

## License

This reference implementation is provided under the MIT license. See the repository root for more
information.
