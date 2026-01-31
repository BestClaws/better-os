//! Input service tasks: hardware readers and routing dispatcher.
//!
//! This module exposes embassy tasks that bridge HAL input devices into the
//! system-wide input pipeline. Raw device samples are normalized into
//! `HighLevelEvent`s and pushed through the shared input bus before being
//! routed to the system UI or active applications.

mod dispatcher;
mod readers;

pub use dispatcher::input_dispatcher_task;
pub use readers::{button_reader_task, encoder_reader_task, touch_reader_task};
