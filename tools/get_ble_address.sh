#!/bin/bash
# Get local BLE adapter address

echo "=== BLE Adapter Information ==="
echo ""

# Try hciconfig first
if command -v hciconfig &> /dev/null; then
    echo "Using hciconfig:"
    hciconfig | grep -A 3 "hci0"
    echo ""
fi

# Try bluetoothctl
if command -v bluetoothctl &> /dev/null; then
    echo "Using bluetoothctl:"
    echo "show" | bluetoothctl | grep -E "Controller|Address"
    echo ""
fi

# Try btmgmt
if command -v btmgmt &> /dev/null; then
    echo "Using btmgmt:"
    sudo btmgmt info
    echo ""
fi

echo "=== Instructions ==="
echo "1. Look for 'BD Address' or 'Address' in the output above"
echo "2. Copy the MAC address (format: XX:XX:XX:XX:XX:XX)"
echo "3. Update src/system/services/hps_service.rs line 89:"
echo "   let hps_server_addr: [u8; 6] = [0xXX, 0xXX, 0xXX, 0xXX, 0xXX, 0xXX];"
echo ""
echo "Note: The byte order should match the address exactly"
