use alloc::boxed::Box;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::mutex::Mutex;
use crate::system::hal::display::AsyncDisplay;
use crate::system::services::human_input::{HumanInputEvent, HUMAN_INPUT_CH};
use crate::system::ui::compositor::UICompositor;
use defmt::warn;

#[embassy_executor::task]
pub async fn compositor_service(
    display: &'static Mutex<CriticalSectionRawMutex, Box<dyn AsyncDisplay>>,
) {
    let mut compositor = UICompositor::new();

}
