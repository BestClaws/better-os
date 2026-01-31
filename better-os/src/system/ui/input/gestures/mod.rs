mod event;
mod result;
mod traits;

pub mod core;
pub mod types;

pub use event::PointerEvent;
pub use result::GestureResult;
pub use traits::PointerGesture;

pub use types::edge_swipe::{EdgeSwipeRecognizer, SwipeConfig, SwipeGestureUpdate};
