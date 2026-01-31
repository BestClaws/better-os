//! UI compositor module: orchestrates window composition, animations, and input glue.
//!
//! This module is split for clarity:
//! - `core`: `UICompositor` state machine and composition pipeline
//! - `blitter`: high-performance surface blits (format-agnostic via BPP)
//! - `strategy`: update strategy selection helpers
//! - `animation`: animation traits, configs, and easing functions
//! - `region`: helpers for sub-buffer extraction

pub mod animation;
pub mod blitter;
pub mod core;
pub mod region;
pub mod strategy;

pub use animation::TransitionDirection;
pub use animation::{
    ease_in_out_circular, ease_in_out_cubic, ease_out_bounce, AnimationConfig, FadeAnimation,
    SlideZoomAnimation, WindowAnimation,
};
pub use blitter::SurfaceBlitter;
pub use core::UICompositor;
