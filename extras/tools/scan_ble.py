#!/usr/bin/env python3
"""
Simple BLE scanner to verify HPS server is advertising
"""
import asyncio
from bleak import BleakScanner

async def scan_for_hps():
    print("Scanning for BLE devices...")
    print("Looking for HPS service UUID: 00001823-0000-1000-8000-00805f9b34fb")
    print()
    
    devices = await BleakScanner.discover(timeout=10.0, return_adv=True)
    
    hps_uuid = "00001823-0000-1000-8000-00805f9b34fb"
    
    for address, (device, adv_data) in devices.items():
        print(f"Device: {device.name or 'Unknown'}")
        print(f"  Address: {address}")
        print(f"  Address Type: {device.address_type if hasattr(device, 'address_type') else 'N/A'}")
        print(f"  RSSI: {adv_data.rssi}")
        print(f"  Service UUIDs: {adv_data.service_uuids}")
        
        # Check if HPS service is advertised
        if hps_uuid in [str(uuid).lower() for uuid in adv_data.service_uuids]:
            print(f"  *** FOUND HPS SERVER! ***")
            addr_bytes = address.replace(':', '').upper()
            byte_array = ', '.join([f"0x{addr_bytes[i:i+2]}" for i in range(0, 12, 2)])
            print(f"  Device config: [{byte_array}]")
        
        print()

if __name__ == "__main__":
    asyncio.run(scan_for_hps())
