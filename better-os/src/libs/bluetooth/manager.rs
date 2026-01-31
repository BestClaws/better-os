use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::channel::{Channel, Receiver, Sender};

use crate::libs::bluetooth::types::{BluetoothCommand, BluetoothError, BluetoothEvent};

const COMMAND_CAPACITY: usize = 4;
const EVENT_CAPACITY: usize = 32;

static COMMAND_CHANNEL: Channel<CriticalSectionRawMutex, BluetoothCommand, COMMAND_CAPACITY> =
    Channel::new();
static EVENT_CHANNEL: Channel<CriticalSectionRawMutex, BluetoothEvent, EVENT_CAPACITY> =
    Channel::new();

/// Handle for interacting with the system Bluetooth stack.
#[derive(Clone, Copy, Debug, Default)]
pub struct Bluetooth;

/// Stream of Bluetooth events produced by the system stack.
pub struct BluetoothEvents {
    inner: Receiver<'static, CriticalSectionRawMutex, BluetoothEvent, EVENT_CAPACITY>,
}

/// Acquire a handle to the shared Bluetooth stack.
pub fn bluetooth() -> Bluetooth {
    Bluetooth
}

impl Bluetooth {
    /// Request the Bluetooth stack to begin scanning for nearby devices.
    pub async fn start_scan(&self) -> Result<(), BluetoothError> {
        COMMAND_CHANNEL.send(BluetoothCommand::StartScan).await;
        Ok(())
    }

    /// Request the Bluetooth stack to stop any active scan session.
    pub async fn stop_scan(&self) -> Result<(), BluetoothError> {
        COMMAND_CHANNEL.send(BluetoothCommand::StopScan).await;
        Ok(())
    }

    /// Subscribe to Bluetooth events emitted by the system stack.
    pub fn events(&self) -> BluetoothEvents {
        BluetoothEvents {
            inner: EVENT_CHANNEL.receiver(),
        }
    }
}

impl BluetoothEvents {
    /// Await the next Bluetooth event from the system stack.
    pub async fn recv(&mut self) -> BluetoothEvent {
        self.inner.receive().await
    }
}

pub(crate) type CommandReceiver =
    Receiver<'static, CriticalSectionRawMutex, BluetoothCommand, COMMAND_CAPACITY>;
pub(crate) type EventSender =
    Sender<'static, CriticalSectionRawMutex, BluetoothEvent, EVENT_CAPACITY>;

pub(crate) fn command_receiver() -> CommandReceiver {
    COMMAND_CHANNEL.receiver()
}

pub(crate) fn event_sender() -> EventSender {
    EVENT_CHANNEL.sender()
}
