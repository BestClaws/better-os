# HTTP Over BLE Architecture

This document describes the clean architecture for HTTP requests over BLE using the HTTP Proxy Service (HPS).

## Architecture Overview

```
┌─────────────────────────────────────────────────────────────┐
│                        Applications                          │
│                     (src/apps/*.rs)                          │
│  - Uses clean reqwest-like API: http::get().send().await    │
└─────────────────┬───────────────────────────────────────────┘
                  │
                  │ use libs::http::Client
                  ▼
┌─────────────────────────────────────────────────────────────┐
│              HTTP Client Library (libs/http)                 │
│  - Client: http::Client::new()                              │
│  - RequestBuilder: .get().header().send()                   │
│  - Response: .status(), .text(), .json()                    │
│  - Error: Clean error types                                 │
└─────────────────┬───────────────────────────────────────────┘
                  │
                  │ Channel: HttpServiceRequest
                  ▼
┌─────────────────────────────────────────────────────────────┐
│         HTTP Service (services/http_service.rs)              │
│  - Bridges HTTP client API to HPS protocol                  │
│  - Handles request/response translation                     │
│  - Provides clean separation of concerns                    │
└─────────────────┬───────────────────────────────────────────┘
                  │
                  │ Channel: HpsRequest
                  ▼
┌─────────────────────────────────────────────────────────────┐
│          HPS Service (services/hps_service.rs)               │
│  - BLE connection management (scanning, connecting)         │
│  - GATT operations (discover, read, write, notify)          │
│  - HPS protocol implementation (URI, Control, Status, Body) │
└─────────────────┬───────────────────────────────────────────┘
                  │
                  │ BLE GATT over trouble-host
                  ▼
┌─────────────────────────────────────────────────────────────┐
│              BLE HPS Server (Python/Phone)                   │
│  - Receives BLE GATT requests                               │
│  - Executes HTTP requests on behalf of client               │
│  - Returns responses via GATT notifications                 │
└─────────────────────────────────────────────────────────────┘
```

## Module Structure

### 1. **src/libs/http/** - User-Facing HTTP Library

Clean, reqwest-inspired API for applications:

```rust
use crate::libs::http;

// Simple GET request
let response = http::get("https://api.example.com/data").await?;
let text = response.text()?;

// With client
let client = http::Client::new();
let response = client
    .get_secure("https://api.example.com/data")
    .header("Authorization", "Bearer token")
    .send()
    .await?;
    
if response.is_success() {
    let status = response.status();  // u16
    let body = response.bytes();      // &[u8]
}
```

**Files:**
- `mod.rs` - Public API exports
- `client.rs` - HTTP Client with method shortcuts
- `request.rs` - RequestBuilder for chaining
- `response.rs` - Response type with helper methods
- `error.rs` - Clean error types

### 2. **src/system/services/http_service.rs** - HTTP Abstraction Layer

Bridges the HTTP client API to the HPS protocol:
- Receives `HttpServiceRequest` from apps via channels
- Translates to `HpsRequest` for HPS service
- Returns `HttpServiceResponse` to apps
- Provides clean separation between HTTP and BLE concerns

### 3. **src/system/services/hps_service.rs** - HPS Protocol & BLE Layer

Implements HPS v1.0 specification over BLE:
- **BLE Management**: Scanning, connecting, handling disconnections
- **GATT Operations**: Service discovery, characteristic operations
- **HPS Protocol**: 
  - Write URI characteristic
  - Write HTTP Control Point (triggers request)
  - Read Status Code (with notification)
  - Read Entity Body
- **Request Queue**: Processes requests sequentially

### 4. **src/libs/hps/** - HPS Protocol Types

Protocol definitions and types:
- `types.rs` - HpsUuids, HttpMethod, HttpRequest, HttpResponse, DataStatus
- `error.rs` - HpsError enum
- `client.rs` - Legacy (deprecated, kept for reference)
- `gatt.rs` - Legacy (deprecated, operations moved to hps_service)

## Request Flow Example

```rust
// App code (discord.rs)
let response = http::Client::new()
    .get_secure("https://jsonplaceholder.typicode.com/posts")
    .send()
    .await?;

// Flow:
// 1. RequestBuilder creates HttpServiceRequest
// 2. Sent to http_service via channel
// 3. http_service translates to HpsRequest
// 4. Sent to hps_service via channel
// 5. hps_service performs GATT operations:
//    - Write URI: "jsonplaceholder.typicode.com/posts"
//    - Write Control Point: 0x06 (GET Secure)
//    - Wait for Status notification
//    - Read Status Code
//    - Read Entity Body
// 6. Response flows back through channels
// 7. App receives http::Response
```

## Key Design Decisions

### Separation of Concerns
- **libs/http**: HTTP semantics only, no BLE knowledge
- **http_service**: Translation layer, no direct BLE access
- **hps_service**: BLE/GATT only, no HTTP semantics beyond HPS protocol

### Channel-Based Communication
- Services communicate via Embassy channels
- Each layer is independent and testable
- Clean boundaries prevent tight coupling

### Stateless HTTP Client
- `http::Client` has no internal state
- All state lives in `http_service` and `hps_service` tasks
- Multiple clients can share the same underlying connection

### Error Handling
- Each layer has its own error types
- Errors are converted at layer boundaries
- Apps see clean `http::Error`, not BLE errors

## API Comparison: Old vs New

### Old API (Channel-based HpsClient)
```rust
let mut client = HpsClient::new();
let request = HttpRequest {
    method: HttpMethod::GetSecure,
    uri: "jsonplaceholder.typicode.com/posts",
    headers: "",
    body: &[],
};
let response = client.send_request(request).await?;
```

### New API (reqwest-like)
```rust
let client = http::Client::new();
let response = client
    .get_secure("https://jsonplaceholder.typicode.com/posts")
    .send()
    .await?;
```

## Benefits

1. **Clean API**: Familiar reqwest-style interface
2. **Separation**: HTTP concerns separated from BLE concerns
3. **Testability**: Each layer can be tested independently
4. **Maintainability**: Clear responsibilities per module
5. **Reusability**: BLE stack can be used for other services
6. **Extensibility**: Easy to add features (headers, body, etc.)

## Files Summary

### Created/Modified
- ✅ `src/libs/http/` - New HTTP client library (5 files)
- ✅ `src/system/services/http_service.rs` - New HTTP service
- ✅ `src/apps/discord.rs` - Updated to use new API
- ✅ `src/system/services/hps_service.rs` - Unchanged (already has GATT ops)
- ✅ `src/libs/hps/mod.rs` - Updated exports and documentation

### Removed
- ❌ `src/system/services/bluetooth_service.rs` - Old service (deleted)

### Kept (For Reference/Legacy)
- 📦 `src/libs/hps/client.rs` - Old channel-based client (deprecated)
- 📦 `src/libs/hps/gatt.rs` - GATT stubs (deprecated)
- 📦 `src/libs/bluetooth/` - Used by bluetooth_scanner app (keep)

## Future Improvements

1. **Buffer Size**: Increase from 250 to 512 bytes for full responses
2. **Chunked Transfer**: Support for large responses via multiple reads
3. **Request Headers**: Add proper HTTP header support
4. **Connection Pool**: Manage multiple BLE connections
5. **Caching**: Cache responses for repeated requests
6. **Retry Logic**: Automatic retry on transient failures
