mod manager;
mod types;
mod util;

pub use manager::{bluetooth, Bluetooth, BluetoothEvents};
pub use types::{
    BlePacket, BluetoothError, BluetoothEvent, DeviceName, DiscoveredDevice, GattServiceStatus,
    ScanStatus, BLE_PACKET_CAPACITY,
};

pub(crate) use manager::{command_receiver, event_sender, CommandReceiver, EventSender};
pub(crate) use types::BluetoothCommand;
pub(crate) use util::parse_device_name;
