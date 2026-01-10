#!/bin/bash
echo "Checking if BLE advertising is active..."
echo "Run this while the Python HPS server is running"
echo ""
timeout 15 bluetoothctl <<EOF
scan on
EOF
