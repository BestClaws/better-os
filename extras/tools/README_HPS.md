# HPS Server - Python BLE Peripheral

Python implementation of Bluetooth SIG HTTP Proxy Service v1.0 specification.

## Overview

This server acts as a BLE peripheral that exposes HTTP proxy functionality via GATT. It receives HTTP requests from BLE clients (like the better-os device) and forwards them to actual HTTP servers.

## Requirements

```bash
pip install bless requests
```

**Note:** `bless` library works best on Linux. For other platforms, you may need alternative BLE libraries.

## Usage

### Basic Usage
```bash
python hps_server.py
```

The server will:
1. Start advertising as "BetterOS_HPS"
2. Expose HPS GATT service (UUID 0x1823)
3. Wait for BLE connections
4. Process HTTP requests and return responses

### Testing with better-os

1. **Start the Python server:**
   ```bash
   cd extras/tools
   python hps_server.py
   ```

2. **Get the server's BLE address:**
   - The server will show its MAC address in the log
   - On Linux: Check with `bluetoothctl` or `hciconfig`
   - Example: `C0:8F:1A:05:E4:FF`

3. **Update better-os HPS server address:**
   Edit `src/system/services/hps_service.rs`:
   ```rust
   let hps_server_addr: [u8; 6] = [0xC0, 0x8F, 0x1A, 0x05, 0xE4, 0xFF];
   ```

4. **Build and flash better-os:**
   ```bash
   cargo build --release
   # Flash to device
   ```

5. **Monitor logs:**
   - Python server: Shows incoming requests and HTTP responses
   - Device logs: Shows BLE connection, service discovery, GATT operations

## HPS Characteristics

The server implements all 6 mandatory characteristics:

| Characteristic | UUID | Properties | Purpose |
|---------------|------|------------|---------|
| URI | 0x2AB6 | Write, Write Long | HTTP request URI |
| HTTP Headers | 0x2AB7 | Read, Write | HTTP headers |
| HTTP Status Code | 0x2AB8 | Read, Notify | Response status |
| HTTP Entity Body | 0x2AB9 | Read, Write | Request/response body |
| HTTP Control Point | 0x2ABA | Write | Trigger HTTP request |
| HTTPS Security | 0x2ABB | Read | HTTPS support (0x00 = none) |

## HTTP Methods Supported

- 0x01: GET
- 0x02: HEAD
- 0x03: POST
- 0x04: PUT
- 0x05: DELETE
- 0x06: GET (HTTPS)
- 0x07: HEAD (HTTPS)
- 0x08: POST (HTTPS)
- 0x09: PUT (HTTPS)
- 0x0A: DELETE (HTTPS)
- 0x0B: CANCEL

## Request Flow

1. Client writes URI to URI characteristic
2. Client writes headers to HTTP Headers characteristic (optional)
3. Client writes body to HTTP Entity Body characteristic (optional)
4. Client enables notifications on HTTP Status Code CCCD
5. Client writes method opcode to HTTP Control Point (triggers request)
6. Server makes HTTP request to specified URI
7. Server sends status code notification
8. Client reads HTTP Headers characteristic
9. Client reads HTTP Entity Body characteristic

## Limitations

- Maximum size per characteristic: 512 bytes (per HPS v1.0 spec)
- Headers/body larger than 512 bytes are truncated (data_status indicates truncation)
- Single concurrent request (new request cancels previous)
- No authentication/authorization (open proxy)
- Basic HTTPS support (certificate validation enabled)

## Troubleshooting

### Server won't start
- Check if `bless` is installed: `pip install bless`
- Ensure Bluetooth adapter is available
- Try running with sudo on Linux: `sudo python hps_server.py`

### Device won't connect
- Verify BLE address matches in `hps_service.rs`
- Check address type (RANDOM vs PUBLIC) matches
- Ensure server is advertising before device tries to connect
- Check Bluetooth is enabled on host machine

### HTTP requests fail
- Check server logs for error messages
- Verify URI format (should include protocol or server adds http://)
- Test the target URL works: `curl http://example.com`
- Check firewall/network settings

## Development

### Modify server behavior
Edit `hps_server.py`:
- `execute_http_request()`: Change HTTP request logic
- `read_request()`: Modify response data format
- `write_request()`: Change request handling

### Add features
- Authentication: Check client device address before processing
- Request queue: Store multiple requests
- WebSocket support: Upgrade HTTP connections
- Custom headers: Add proxy identification headers

## Examples

### Simple GET request
```python
# Client side (better-os):
let request = HttpRequest {
    method: HttpMethod::Get,
    uri: "example.com/api/data",
    headers: "",
    body: &[],
};
let response = hps_client.send_request(request).await?;
```

### POST with JSON
```python
# Client side:
let request = HttpRequest {
    method: HttpMethod::Post,
    uri: "api.example.com/data",
    headers: "Content-Type: application/json",
    body: b"{\"key\":\"value\"}",
};
```

## License

MIT License - See main project LICENSE file
