use defmt::info;
use embassy_executor::Spawner;
use embassy_time::Instant;

// TODO: why is there a reference to driver in the task? get rid of this.
#[embassy_executor::task]
pub async fn task_spawner_service(spawner: Spawner) {
    info!("[{}s] task spawner service started", Instant::now().as_millis() as f32 / 1000f32);


}





