use heapless::String;

/// Maximum UTF-8 bytes stored for a device name.
pub const MAX_DEVICE_NAME_LEN: usize = 32;

/// Heap-allocated string used for device names without relying on a system allocator.
pub type DeviceName = String<MAX_DEVICE_NAME_LEN>;

/// Commands issued to the Bluetooth service task.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BluetoothCommand {
    StartScan,
    StopScan,
}

/// High-level errors surfaced by the Bluetooth framework.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BluetoothError {
    ChannelClosed,
    OperationFailed,
}

/// Lifecycle states for Bluetooth scanning operations.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ScanStatus {
    Idle,
    Starting,
    Running,
    Stopping,
    AlreadyRunning,
    Failed(BluetoothError),
}

/// Bluetooth events emitted by the background service.
#[derive(Clone, Debug)]
pub enum BluetoothEvent {
    ScanStatus(ScanStatus),
    DeviceDiscovered(DiscoveredDevice),
}

/// Description of a discovered Bluetooth device.
#[derive(Clone, Debug)]
pub struct DiscoveredDevice {
    pub address: [u8; 6],
    pub name: Option<DeviceName>,
    pub rssi: i8,
}

impl DiscoveredDevice {
    pub fn new(address: [u8; 6], name: Option<DeviceName>, rssi: i8) -> Self {
        Self { address, name, rssi }
    }

    pub fn name(&self) -> Option<&str> {
        self.name.as_deref()
    }
}
