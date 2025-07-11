#![no_std]
#![no_main]

use embassy_executor::Spawner;
use embassy_time::Timer;

mod system;
mod apps;
mod util;

extern crate alloc;

#[esp_hal_embassy::main]
async fn main(spawner: Spawner)  {
        system::kernel::start::start(spawner);
        Timer::after_secs(60 * 60 * 24 * 365 * 10).await;
}




