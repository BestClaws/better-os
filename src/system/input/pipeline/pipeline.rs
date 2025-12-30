use defmt::trace;

use crate::system::input::RawInputQueue;

use super::EventRouter;

/// Drives the end-to-end input event pipeline by pulling from the shared queue
/// and delegating routing decisions to an [`EventRouter`].
pub struct EventPipeline {
    router: EventRouter,
}

impl EventPipeline {
    pub fn new(router: EventRouter) -> Self {
        Self { router }
    }

    pub async fn pump_once(&self) {
        let event = RawInputQueue::pop().await;
        trace!("dispatching input event");
        self.router.route(event).await;
    }

    pub async fn run(&self) -> ! {
        loop {
            self.pump_once().await;
        }
    }

    pub fn router(&self) -> &EventRouter {
        &self.router
    }
}
