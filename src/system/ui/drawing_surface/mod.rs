//! Drawing surface module
//!
//! Space-grade 2D surface with built-in dirty region tracking and
//! format-agnostic rasterization front-end. The surface stores pixels in the
//! negotiated display `PixelFormat` (currently reference implementation: RGB565,
//! hi,lo byte order) and exposes both legacy `Rasterizer` API and new `DrawTarget` API.
//!
//! Key properties:
//! - Negotiated format storage with per-format hot-path function pointers
//! - LVGL-style dirty region tracking with coalescing and configurable cap
//! - Batch mode that accumulates a single dirty region per operation
//! - Zero virtual dispatch in inner loops
//!
//! Typical usage (window manager provides format and buffer):
//! ```ignore
//! use crate::system::hal::display::PixelFormat;
//! use crate::system::ui::drawing_surface::DrawingSurface;
//! use crate::libs::gfx::Layer;
//!
//! // Negotiated elsewhere:
//! let width: u32 = 240;
//! let height: u32 = 240;
//! let format: PixelFormat = PixelFormat::Rgb565;
//! let mut framebuffer: &mut [u8] = /* ... */;
//!
//! // Create and attach buffer
//! let mut surface = DrawingSurface::new_unattached(width, height, format);
//! surface.attach_buffer(framebuffer);
//!
//! // Draw via new Layer API
//! let mut layer = Layer::from_draw_target(&mut surface);
//! ```

pub mod draw_target_impl;
pub mod surface;
pub mod util;

pub use surface::DrawingSurface;
