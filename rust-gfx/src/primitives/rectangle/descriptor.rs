use crate::color::Rgba8888;
use crate::types::{BorderSide, Gradient, Opa, OPA_COVER};

/// Configuration for rendering an LVGL-compatible rectangle.
///
/// This mirrors the subset of `lv_draw_rect_dsc_t` that the renderer relies on
/// while keeping defaults that are convenient in Rust.
#[derive(Clone, Debug)]
pub struct RectDsc {
    pub bg_color: Rgba8888,
    pub bg_opa: Opa,
    pub bg_grad: Gradient,
    pub radius: i32,

    pub border_color: Rgba8888,
    pub border_opa: Opa,
    pub border_width: i32,
    pub border_side: BorderSide,

    pub shadow_color: Rgba8888,
    pub shadow_opa: Opa,
    pub shadow_width: i32,
    pub shadow_offset_x: i32,
    pub shadow_offset_y: i32,
    pub shadow_spread: i32,

    pub outline_color: Rgba8888,
    pub outline_opa: Opa,
    pub outline_width: i32,
    pub outline_pad: i32,
}

impl RectDsc {
    /// Create a descriptor with LVGL-style defaults.
    pub fn new() -> Self {
        Self {
            bg_color: Rgba8888::WHITE,
            bg_opa: OPA_COVER,
            bg_grad: Gradient::none(),
            radius: 0,

            border_color: Rgba8888::BLACK,
            border_opa: 0,
            border_width: 0,
            border_side: BorderSide::FULL,

            shadow_color: Rgba8888::BLACK,
            shadow_opa: 0,
            shadow_width: 0,
            shadow_offset_x: 0,
            shadow_offset_y: 0,
            shadow_spread: 0,

            outline_color: Rgba8888::BLACK,
            outline_opa: 0,
            outline_width: 0,
            outline_pad: 0,
        }
    }
}

impl Default for RectDsc {
    fn default() -> Self {
        Self::new()
    }
}
