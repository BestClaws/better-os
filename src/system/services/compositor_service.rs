use crate::system::ui::compositor::Compositor;
use crate::system::apps::app_spawner_service::{get_window_for_app};
use crate::system::services::input::{HUMAN_INPUT_CHANNEL, HumanInput};

const MAX_APPS: usize = 5;

#[embassy_executor::task]
pub async fn compositor_service_task() {
    let mut temp_buf = [0u8; 1024]; // temporary buffer for transitions

    let mut compositor = Compositor::new(128, 64, &mut temp_buf);
    let mut current_app: usize = 0;
    let mut split_view: bool = false;

    // Render initial
    if let Some(window) = get_window_for_app(current_app).await {
        compositor.render_single_window(&window).await;
    }

    let input_rx = HUMAN_INPUT_CHANNEL.receiver();

    loop {
        let input = input_rx.receive().await;

        match input {
            HumanInput::NavUp => {
                let new_app = (current_app + 1) % MAX_APPS;

                if let (Some(from), Some(to)) = (
                    get_window_for_app(current_app).await,
                    get_window_for_app(new_app).await,
                ) {
                    compositor.animate_transition(&from, &to).await;
                    current_app = new_app;
                }
            }

            HumanInput::NavDown => {
                let new_app = (current_app + MAX_APPS - 1) % MAX_APPS;

                if let (Some(from), Some(to)) = (
                    get_window_for_app(current_app).await,
                    get_window_for_app(new_app).await,
                ) {
                    compositor.animate_transition(&from, &to).await;
                    current_app = new_app;
                }
            }

            HumanInput::Ok => {
                split_view = !split_view;

                if split_view {
                    let next_app = (current_app + 1) % MAX_APPS;

                    if let (Some(left), Some(right)) = (
                        get_window_for_app(current_app).await,
                        get_window_for_app(next_app).await,
                    ) {
                        compositor.render_split(&left, &right).await;
                    }
                } else {
                    if let Some(window) = get_window_for_app(current_app).await {
                        compositor.render_single_window(&window).await;
                    }
                }
            }

            _ => {}
        }
    }
}
