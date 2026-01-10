#!/usr/bin/env python3
"""
HPS (HTTP Proxy Service) Server - BLE Peripheral
Implements Bluetooth SIG HPS v1.0 specification

This server acts as a BLE peripheral that exposes HTTP proxy functionality
via GATT characteristics. It receives HTTP requests from BLE clients and
forwards them to actual HTTP servers.

Requirements:
    pip install bleak requests

Usage:
    python hps_server.py
"""

import asyncio
import struct
import logging
from typing import Optional
from bleak import BleakGATTCharacteristic, BleakGATTServiceCollection
from bleak.backends.characteristic import GattCharacteristicsFlags
import requests

try:
    from bless import (  # type: ignore
        BlessServer,
        BlessGATTCharacteristic,
        GATTCharacteristicProperties,
        GATTAttributePermissions
    )
except ImportError:
    print("ERROR: bless library not found")
    print("Install with: pip install bless")
    exit(1)

logging.basicConfig(level=logging.INFO)
logger = logging.getLogger(__name__)

# HPS UUIDs (Bluetooth SIG assigned numbers)
HPS_SERVICE_UUID = "00001823-0000-1000-8000-00805f9b34fb"
URI_CHAR_UUID = "00002ab6-0000-1000-8000-00805f9b34fb"
HTTP_HEADERS_CHAR_UUID = "00002ab7-0000-1000-8000-00805f9b34fb"
HTTP_STATUS_CODE_CHAR_UUID = "00002ab8-0000-1000-8000-00805f9b34fb"
HTTP_ENTITY_BODY_CHAR_UUID = "00002ab9-0000-1000-8000-00805f9b34fb"
HTTP_CONTROL_POINT_CHAR_UUID = "00002aba-0000-1000-8000-00805f9b34fb"
HTTPS_SECURITY_CHAR_UUID = "00002abb-0000-1000-8000-00805f9b34fb"

# HTTP Methods (HPS opcodes)
HTTP_METHOD_GET = 0x01
HTTP_METHOD_HEAD = 0x02
HTTP_METHOD_POST = 0x03
HTTP_METHOD_PUT = 0x04
HTTP_METHOD_DELETE = 0x05
HTTP_METHOD_GET_SECURE = 0x06
HTTP_METHOD_HEAD_SECURE = 0x07
HTTP_METHOD_POST_SECURE = 0x08
HTTP_METHOD_PUT_SECURE = 0x09
HTTP_METHOD_DELETE_SECURE = 0x0A
HTTP_METHOD_CANCEL = 0x0B

METHOD_MAP = {
    HTTP_METHOD_GET: "GET",
    HTTP_METHOD_HEAD: "HEAD",
    HTTP_METHOD_POST: "POST",
    HTTP_METHOD_PUT: "PUT",
    HTTP_METHOD_DELETE: "DELETE",
    HTTP_METHOD_GET_SECURE: "GET",  # HTTPS
    HTTP_METHOD_HEAD_SECURE: "HEAD",
    HTTP_METHOD_POST_SECURE: "POST",
    HTTP_METHOD_PUT_SECURE: "PUT",
    HTTP_METHOD_DELETE_SECURE: "DELETE",
}

# Data Status bits
DATA_STATUS_HEADERS_RECEIVED = 0x01
DATA_STATUS_HEADERS_TRUNCATED = 0x02
DATA_STATUS_BODY_RECEIVED = 0x04
DATA_STATUS_BODY_TRUNCATED = 0x08


class HPSServer:
    """HTTP Proxy Service BLE Server"""
    
    def __init__(self, name: str = "HPS_Proxy"):
        self.name = name
        self.server: Optional[BlessServer] = None
        self.connected_device = None
        
        # HPS state
        self.uri: str = ""
        self.headers: str = ""
        self.body: bytes = b""
        self.status_code: int = 0
        self.response_headers: str = ""
        self.response_body: bytes = b""
        self.data_status: int = 0
        
        # Notification subscriptions
        self.status_code_subscribed = False
        
    async def setup(self):
        """Initialize BLE server and GATT services"""
        logger.info(f"========================================")
        logger.info(f"Setting up HPS server: {self.name}")
        logger.info(f"========================================")
        
        self.server = BlessServer(name=self.name, name_overwrite=True)
        logger.info(f"BlessServer created")
        
        # Set connection callbacks
        self.server.connection_callback = self.on_connection
        self.server.disconnection_callback = self.on_disconnection
        logger.info(f"Connection callbacks registered")
        
        # Add HPS Service - this SHOULD make it advertise the service UUID
        logger.info(f"Adding HPS service: {HPS_SERVICE_UUID}")
        await self.server.add_new_service(HPS_SERVICE_UUID)
        logger.info(f"HPS service added successfully")
        
        # 1. URI Characteristic (Write, Write Long)
        logger.info(f"Adding URI characteristic: {URI_CHAR_UUID}")
        await self.server.add_new_characteristic(
            HPS_SERVICE_UUID,
            URI_CHAR_UUID,
            GATTCharacteristicProperties.write,
            None,
            GATTAttributePermissions.writeable
        )
        
        # 2. HTTP Headers Characteristic (Read, Read Long, Write, Write Long)
        await self.server.add_new_characteristic(
            HPS_SERVICE_UUID,
            HTTP_HEADERS_CHAR_UUID,
            GATTCharacteristicProperties.read | GATTCharacteristicProperties.write,
            None,
            GATTAttributePermissions.readable | GATTAttributePermissions.writeable
        )
        
        # 3. HTTP Status Code Characteristic (Read, Notify)
        await self.server.add_new_characteristic(
            HPS_SERVICE_UUID,
            HTTP_STATUS_CODE_CHAR_UUID,
            GATTCharacteristicProperties.read | GATTCharacteristicProperties.notify,
            None,
            GATTAttributePermissions.readable
        )
        
        # 4. HTTP Entity Body Characteristic (Read, Read Long, Write, Write Long)
        await self.server.add_new_characteristic(
            HPS_SERVICE_UUID,
            HTTP_ENTITY_BODY_CHAR_UUID,
            GATTCharacteristicProperties.read | GATTCharacteristicProperties.write,
            None,
            GATTAttributePermissions.readable | GATTAttributePermissions.writeable
        )
        
        # 5. HTTP Control Point Characteristic (Write)
        await self.server.add_new_characteristic(
            HPS_SERVICE_UUID,
            HTTP_CONTROL_POINT_CHAR_UUID,
            GATTCharacteristicProperties.write,
            None,
            GATTAttributePermissions.writeable
        )
        
        # 6. HTTPS Security Characteristic (Read)
        await self.server.add_new_characteristic(
            HPS_SERVICE_UUID,
            HTTPS_SECURITY_CHAR_UUID,
            GATTCharacteristicProperties.read,
            bytearray([0x00]),  # No HTTPS security (basic proxy)
            GATTAttributePermissions.readable
        )
        
        # Set up write callbacks
        self.server.read_request_func = self.read_request
        self.server.write_request_func = self.write_request
        
        logger.info("========================================")
        logger.info("HPS server setup complete - all characteristics added")
        logger.info("========================================")
        
    def on_connection(self, device_address):
        """Called when a client connects"""
        logger.info(f"")
        logger.info(f"★★★★★★★★★★★★★★★★★★★★★★★★★★★★★★★★★★★★★★")
        logger.info(f"★★★ CLIENT CONNECTED: {device_address} ★★★")
        logger.info(f"★★★★★★★★★★★★★★★★★★★★★★★★★★★★★★★★★★★★★★")
        logger.info(f"")
        self.connected_device = device_address
        
    def on_disconnection(self, device_address):
        """Called when a client disconnects"""
        logger.info(f"")
        logger.info(f"★★★★★★★★★★★★★★★★★★★★★★★★★★★★★★★★★★★★★★")
        logger.info(f"★★★ CLIENT DISCONNECTED: {device_address} ★★★")
        logger.info(f"★★★★★★★★★★★★★★★★★★★★★★★★★★★★★★★★★★★★★★")
        logger.info(f"")
        self.connected_device = None
        
    def read_request(self, characteristic: BleakGATTCharacteristic, **kwargs) -> bytearray:
        """Handle read requests - SYNCHRONOUS (bless doesn't await)"""
        uuid = characteristic.uuid.lower()
        logger.info(f"")
        logger.info(f"=== GATT Read Request ===")
        logger.info(f"Characteristic: {uuid}")
        
        if uuid == HTTP_STATUS_CODE_CHAR_UUID.lower():
            # Return 3-byte status code: uint16 status + uint8 data_status
            response = struct.pack('<HB', self.status_code, self.data_status)
            logger.info(f"Returning status code: {self.status_code}, data_status: 0x{self.data_status:02x}")
            return bytearray(response)
            
        elif uuid == HTTP_HEADERS_CHAR_UUID.lower():
            logger.info(f"Returning headers ({len(self.response_headers)} bytes)")
            return bytearray(self.response_headers.encode('utf-8'))
            
        elif uuid == HTTP_ENTITY_BODY_CHAR_UUID.lower():
            logger.info(f"Returning body ({len(self.response_body)} bytes)")
            return bytearray(self.response_body)
            
        elif uuid == HTTPS_SECURITY_CHAR_UUID.lower():
            return bytearray([0x00])
            
        return bytearray()
        
    def write_request(self, characteristic: BleakGATTCharacteristic, value: bytearray, **kwargs):
        """Handle write requests - SYNCHRONOUS (bless doesn't await)"""
        uuid = characteristic.uuid.lower()
        logger.info(f"")
        logger.info(f"=== GATT Write Request ===")
        logger.info(f"Characteristic: {uuid}")
        logger.info(f"Value length: {len(value)} bytes")
        
        if uuid == URI_CHAR_UUID.lower():
            self.uri = value.decode('utf-8')
            logger.info(f"✓ URI set to: {self.uri}")
            
        elif uuid == HTTP_HEADERS_CHAR_UUID.lower():
            self.headers = value.decode('utf-8')
            logger.info(f"Headers set: {self.headers}")
            
        elif uuid == HTTP_ENTITY_BODY_CHAR_UUID.lower():
            self.body = bytes(value)
            logger.info(f"Body set ({len(self.body)} bytes)")
            
        elif uuid == HTTP_CONTROL_POINT_CHAR_UUID.lower():
            if len(value) > 0:
                method_opcode = value[0]
                logger.info(f"Control Point triggered with opcode: 0x{method_opcode:02x}")
                # Schedule async task since we can't await in synchronous callback
                import asyncio
                asyncio.create_task(self.execute_http_request(method_opcode))
                
    async def execute_http_request(self, method_opcode: int):
        """Execute HTTP request and send notification"""
        logger.info(f"Executing HTTP request: {METHOD_MAP.get(method_opcode, 'UNKNOWN')}")
        
        if method_opcode == HTTP_METHOD_CANCEL:
            logger.info("Cancel request received")
            return
            
        method = METHOD_MAP.get(method_opcode, "GET")
        is_secure = method_opcode >= HTTP_METHOD_GET_SECURE and method_opcode < HTTP_METHOD_CANCEL
        
        url = self.uri
        if not url.startswith('http'):
            url = f"{'https' if is_secure else 'http'}://{url}"
            
        logger.info(f"Making {method} request to: {url}")
        
        try:
            # Parse headers
            headers_dict = {}
            if self.headers:
                for line in self.headers.split('\n'):
                    if ':' in line:
                        key, value = line.split(':', 1)
                        headers_dict[key.strip()] = value.strip()
            
            # Make HTTP request
            response = requests.request(
                method,
                url,
                headers=headers_dict if headers_dict else None,
                data=self.body if self.body else None,
                timeout=10,
                verify=True
            )
            
            self.status_code = response.status_code
            logger.info(f"HTTP response: {self.status_code}")
            
            # Store response headers (truncate to 4096 bytes - MAX_HEADERS_SIZE)
            response_headers_str = '\n'.join([f"{k}: {v}" for k, v in response.headers.items()])
            if len(response_headers_str) > 4096:
                self.response_headers = response_headers_str[:4096]
                self.data_status = DATA_STATUS_HEADERS_RECEIVED | DATA_STATUS_HEADERS_TRUNCATED
            else:
                self.response_headers = response_headers_str
                self.data_status = DATA_STATUS_HEADERS_RECEIVED
            
            # Store response body (truncate to 5120 bytes - MAX_BODY_SIZE)
            if len(response.content) > 5120:
                self.response_body = response.content[:5120]
                self.data_status |= DATA_STATUS_BODY_RECEIVED | DATA_STATUS_BODY_TRUNCATED
            else:
                self.response_body = response.content
                self.data_status |= DATA_STATUS_BODY_RECEIVED
            
            logger.info(f"Response: {len(self.response_headers)} bytes headers, {len(self.response_body)} bytes body")
            
            # Send notification with status code
            if self.status_code_subscribed:
                status_data = struct.pack('<HB', self.status_code, self.data_status)
                await self.server.update_value(HPS_SERVICE_UUID, HTTP_STATUS_CODE_CHAR_UUID, bytearray(status_data))
                logger.info(f"Sent status code notification: {self.status_code}")
            
        except requests.exceptions.RequestException as e:
            logger.error(f"HTTP request failed: {e}")
            self.status_code = 500
            self.response_headers = ""
            self.response_body = str(e).encode('utf-8')[:5120]
            self.data_status = DATA_STATUS_BODY_RECEIVED
            
    async def run(self):
        """Start the BLE server"""
        await self.setup()
        
        logger.info(f"")
        logger.info(f"========================================")
        logger.info(f"Starting HPS server: {self.name}")
        logger.info(f"Advertising HPS service...")
        logger.info(f"Service UUID: {HPS_SERVICE_UUID}")
        logger.info(f"========================================")
        
        await self.server.start()
        logger.info(f"BLE server started - now advertising!")
        
        # Try to get and display BLE address
        try:
            # The bless library doesn't expose the address directly, 
            # but we can check system bluetooth info
            import subprocess
            result = subprocess.run(['hciconfig'], capture_output=True, text=True)
            if 'BD Address:' in result.stdout:
                for line in result.stdout.split('\n'):
                    if 'BD Address:' in line:
                        addr = line.split('BD Address:')[1].strip().split()[0]
                        logger.info(f"BLE Address: {addr}")
                        # Convert to byte array format for device code
                        addr_bytes = addr.replace(':', '').upper()
                        byte_array = ', '.join([f"0x{addr_bytes[i:i+2]}" for i in range(0, 12, 2)])
                        logger.info(f"Device config: [{byte_array}]")
                        break
        except Exception as e:
            logger.warning(f"Could not detect BLE address: {e}")
            logger.info("Check your system's bluetooth settings for the address")
        
        logger.info("")
        logger.info("========================================")
        logger.info("HPS server is running and waiting for connections...")
        logger.info("Press Ctrl+C to stop.")
        logger.info("========================================")
        logger.info("")
        
        # Poll for connection status since bless doesn't have callbacks in BlueZ backend
        was_connected = False
        try:
            while True:
                is_connected = await self.server.is_connected()
                if is_connected and not was_connected:
                    logger.info("")
                    logger.info("★★★★★★★★★★★★★★★★★★★★★★★★★★★★★★★★★★★★★★")
                    logger.info("★★★ CLIENT CONNECTED ★★★")
                    logger.info("★★★★★★★★★★★★★★★★★★★★★★★★★★★★★★★★★★★★★★")
                    logger.info("")
                    was_connected = True
                elif not is_connected and was_connected:
                    logger.info("")
                    logger.info("★★★★★★★★★★★★★★★★★★★★★★★★★★★★★★★★★★★★★★")
                    logger.info("★★★ CLIENT DISCONNECTED ★★★")
                    logger.info("★★★★★★★★★★★★★★★★★★★★★★★★★★★★★★★★★★★★★★")
                    logger.info("")
                    was_connected = False
                await asyncio.sleep(0.5)
        except KeyboardInterrupt:
            logger.info("")
            logger.info("========================================")
            logger.info("Stopping HPS server...")
            logger.info("========================================")
            await self.server.stop()


async def main():
    """Main entry point"""
    server = HPSServer(name="BetterOS_HPS")
    await server.run()


if __name__ == "__main__":
    asyncio.run(main())
