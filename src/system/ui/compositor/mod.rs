//! UI compositor module: orchestrates window composition, animations, and input glue.
//!
//! This module is split for clarity:
//! - `core`: `UICompositor` state machine and composition pipeline
//! - `blitter`: high-performance surface blits (format-agnostic via BPP)
//! - `strategy`: update strategy selection helpers
//! - `animation`: animation traits, configs, and easing functions
//! - `input`: SUI gesture consumer task and command channel
//! - `region`: helpers for sub-buffer extraction

pub mod core;
pub mod blitter;
pub mod strategy;
pub mod animation;
pub mod input;
pub mod region;

pub use core::UICompositor;
pub use animation::TransitionDirection;
pub use blitter::SurfaceBlitter;
pub use input::{SUI_COMMAND_CH, system_ui_consume_events};
pub use animation::{AnimationConfig, WindowAnimation, SlideZoomAnimation, FadeAnimation,
    ease_in_out_cubic, ease_in_out_circular, ease_out_bounce};

