//! Surface abstraction for rendering
//!
//! A Surface wraps a frame buffer and provides RasterTarget implementation,
//! allowing applications to draw to windows without direct buffer access.

use alloc::vec::Vec;
use alloc::string::String;
use gfx::colors::Color;
use gfx::rasterizer::RasterTarget;
use gfx::rgb565::Rgb565Rasterizer;
use gfx::luma4::Luma4Rasterizer;
use gfx::primitives::font::Font;

use crate::system::window_manager::{WindowGeometry, DirtyRegion};

/// Pixel format for surfaces
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PixelFormat {
    Rgb565,
    Luma4,
}

/// Display metadata for resolution-independent rendering
#[derive(Debug, Clone, Copy)]
pub struct DisplayInfo {
    /// Actual DPI of the display
    pub dpi: f32,
    /// Device pixel ratio (actual_dpi / 160.0)
    pub device_pixel_ratio: f32,
}

impl DisplayInfo {
    /// Baseline DPI for 1:1 device pixel ratio (CSS reference pixel density)
    pub const BASELINE_DPI: f32 = 160.0;

    pub fn new(dpi: f32) -> Self {
        Self {
            dpi,
            device_pixel_ratio: dpi / Self::BASELINE_DPI,
        }
    }

    /// Scale a value by device pixel ratio for resolution-independent sizing
    pub fn scale(&self, logical_pixels: f32) -> f32 {
        logical_pixels * self.device_pixel_ratio
    }
}

/// Window property change request
#[derive(Debug, Clone)]
pub enum WindowRequest {
    /// Set window visibility
    SetVisible(bool),
    /// Change window geometry
    SetGeometry(WindowGeometry),
    /// Set window title
    SetTitle(String),
    /// Request window to front (increase z-index)
    BringToFront,
}

/// A drawing surface backed by a frame buffer
pub struct Surface<'a> {
    buffer: &'a mut [u8],
    width: u16,
    height: u16,
    format: PixelFormat,
    display_info: DisplayInfo,
    window_requests: Vec<WindowRequest>,
    dirty_regions: Vec<DirtyRegion>,
}

impl<'a> Surface<'a> {
    /// Create a new RGB565 surface
    pub fn new_rgb565(buffer: &'a mut [u8], width: u16, height: u16, display_info: DisplayInfo) -> Self {
        Self {
            buffer,
            width,
            height,
            format: PixelFormat::Rgb565,
            display_info,
            window_requests: Vec::new(),
            dirty_regions: Vec::new(),
        }
    }

    /// Create a new LUMA4 surface
    pub fn new_luma4(buffer: &'a mut [u8], width: u16, height: u16, display_info: DisplayInfo) -> Self {
        Self {
            buffer,
            width,
            height,
            format: PixelFormat::Luma4,
            display_info,
            window_requests: Vec::new(),
            dirty_regions: Vec::new(),
        }
    }

    /// Get the pixel format
    pub fn format(&self) -> PixelFormat {
        self.format
    }

    /// Get display information for resolution-independent rendering
    pub fn display_info(&self) -> DisplayInfo {
        self.display_info
    }

    /// Get a rasterizer for this surface
    pub fn rasterizer(&mut self) -> SurfaceRasterizer<'_> {
        match self.format {
            PixelFormat::Rgb565 => {
                SurfaceRasterizer::Rgb565(Rgb565Rasterizer::new(self.buffer, self.width, self.height))
            }
            PixelFormat::Luma4 => {
                SurfaceRasterizer::Luma4(Luma4Rasterizer::new(self.buffer, self.width, self.height))
            }
        }
    }

    /// Clear the surface with a color
    pub fn clear(&mut self, color: Color) {
        let width = self.width;
        let height = self.height;
        match self.rasterizer() {
            SurfaceRasterizer::Rgb565(mut rast) => {
                rast.fill_solid_rect(0, 0, width, height, color);
            }
            SurfaceRasterizer::Luma4(mut rast) => {
                rast.fill_solid_rect(0, 0, width, height, color);
            }
        }
    }
}

/// Enum wrapping different rasterizer types
pub enum SurfaceRasterizer<'a> {
    Rgb565(Rgb565Rasterizer<'a>),
    Luma4(Luma4Rasterizer<'a>),
}

impl<'a> RasterTarget for SurfaceRasterizer<'a> {
    fn width(&self) -> u16 {
        match self {
            SurfaceRasterizer::Rgb565(r) => r.width(),
            SurfaceRasterizer::Luma4(r) => r.width(),
        }
    }

    fn height(&self) -> u16 {
        match self {
            SurfaceRasterizer::Rgb565(r) => r.height(),
            SurfaceRasterizer::Luma4(r) => r.height(),
        }
    }

    fn fill_solid_hspan(&mut self, y: u16, x_start: u16, color: Color, length: u16) {
        match self {
            SurfaceRasterizer::Rgb565(r) => r.fill_solid_hspan(y, x_start, color, length),
            SurfaceRasterizer::Luma4(r) => r.fill_solid_hspan(y, x_start, color, length),
        }
    }

    fn blend_solid_hspan(&mut self, y: u16, x_start: u16, color: Color, coverage: &[u8]) {
        match self {
            SurfaceRasterizer::Rgb565(r) => r.blend_solid_hspan(y, x_start, color, coverage),
            SurfaceRasterizer::Luma4(r) => r.blend_solid_hspan(y, x_start, color, coverage),
        }
    }

    fn blend_color_hspan(&mut self, y: u16, x_start: u16, colors: &[Color], coverage: &[u8]) {
        match self {
            SurfaceRasterizer::Rgb565(r) => r.blend_color_hspan(y, x_start, colors, coverage),
            SurfaceRasterizer::Luma4(r) => r.blend_color_hspan(y, x_start, colors, coverage),
        }
    }

    fn fill_solid_rect(&mut self, x: u16, y: u16, width: u16, height: u16, color: Color) {
        match self {
            SurfaceRasterizer::Rgb565(r) => r.fill_solid_rect(x, y, width, height, color),
            SurfaceRasterizer::Luma4(r) => r.fill_solid_rect(x, y, width, height, color),
        }
    }

    fn fill_solid_vspan(&mut self, x: u16, y_start: u16, color: Color, length: u16) {
        match self {
            SurfaceRasterizer::Rgb565(r) => r.fill_solid_vspan(x, y_start, color, length),
            SurfaceRasterizer::Luma4(r) => r.fill_solid_vspan(x, y_start, color, length),
        }
    }

    fn blend_solid_vspan(&mut self, x: u16, y_start: u16, color: Color, coverage: &[u8]) {
        match self {
            SurfaceRasterizer::Rgb565(r) => r.blend_solid_vspan(x, y_start, color, coverage),
            SurfaceRasterizer::Luma4(r) => r.blend_solid_vspan(x, y_start, color, coverage),
        }
    }

    fn blend_color_vspan(&mut self, x: u16, y_start: u16, colors: &[Color], coverage: &[u8]) {
        match self {
            SurfaceRasterizer::Rgb565(r) => r.blend_color_vspan(x, y_start, colors, coverage),
            SurfaceRasterizer::Luma4(r) => r.blend_color_vspan(x, y_start, colors, coverage),
        }
    }
}

impl<'a> Surface<'a> {
    /// Request a window property change
    pub fn request_window_change(&mut self, request: WindowRequest) {
        self.window_requests.push(request);
    }

    /// Get and clear window requests
    pub fn take_window_requests(&mut self) -> Vec<WindowRequest> {
        core::mem::take(&mut self.window_requests)
    }

    /// Request window to be visible
    pub fn show(&mut self) {
        self.request_window_change(WindowRequest::SetVisible(true));
    }

    /// Request window to be hidden
    pub fn hide(&mut self) {
        self.request_window_change(WindowRequest::SetVisible(false));
    }

    /// Mark a region as dirty (changed)
    pub fn mark_dirty_region(&mut self, x: u16, y: u16, width: u16, height: u16) {
        self.dirty_regions.push(DirtyRegion::new(x, y, width, height));
    }

    /// Mark entire surface as dirty
    pub fn mark_dirty(&mut self) {
        self.dirty_regions.clear();
        self.dirty_regions.push(DirtyRegion::new(0, 0, self.width, self.height));
    }

    /// Get and clear dirty regions
    pub fn take_dirty_regions(&mut self) -> Vec<DirtyRegion> {
        core::mem::take(&mut self.dirty_regions)
    }

    /// Fill a rectangle with a solid color
    pub fn fill_rect(&mut self, x: i16, y: i16, width: u16, height: u16, color: Color) {
        if x < 0 || y < 0 {
            return;
        }
        match self.rasterizer() {
            SurfaceRasterizer::Rgb565(mut rast) => {
                rast.fill_solid_rect(x as u16, y as u16, width, height, color);
            }
            SurfaceRasterizer::Luma4(mut rast) => {
                rast.fill_solid_rect(x as u16, y as u16, width, height, color);
            }
        }
    }

    /// Draw a rectangle outline
    pub fn draw_rect(&mut self, x: i16, y: i16, width: u16, height: u16, color: Color) {
        if x < 0 || y < 0 {
            return;
        }
        let x = x as u16;
        let y = y as u16;
        match self.rasterizer() {
            SurfaceRasterizer::Rgb565(mut rast) => {
                // Top
                rast.fill_solid_rect(x, y, width, 1, color);
                // Bottom
                rast.fill_solid_rect(x, y + height - 1, width, 1, color);
                // Left
                rast.fill_solid_rect(x, y, 1, height, color);
                // Right
                rast.fill_solid_rect(x + width - 1, y, 1, height, color);
            }
            SurfaceRasterizer::Luma4(mut rast) => {
                // Top
                rast.fill_solid_rect(x, y, width, 1, color);
                // Bottom
                rast.fill_solid_rect(x, y + height - 1, width, 1, color);
                // Left
                rast.fill_solid_rect(x, y, 1, height, color);
                // Right
                rast.fill_solid_rect(x + width - 1, y, 1, height, color);
            }
        }
    }

    /// Draw text using embedded font with LRU cache
    pub fn draw_text(&mut self, x: i16, y: i16, text: &str, color: Color) {
        if x < 0 || y < 0 {
            return;
        }

        // Use static mut for persistent font cache (NOT thread-safe, but Surface is not Send)
        static mut FONT_CACHE: Option<Font> = None;
        
        // Embedded UI font
        const UI_FONT_DATA: &[u8] = include_bytes!("../assets/RobotoSlab-Regular.ttf");

        // Initialize font if needed
        let font = unsafe {
            if FONT_CACHE.is_none() {
                FONT_CACHE = Some(Font::builder()
                    .data(UI_FONT_DATA)
                    .size(14.0)
                    .cache("0123456789") // Pre-cache just numbers
                    .max_cache_size(4096)
                    .hint(true)
                    .build());
            }
            FONT_CACHE.as_mut().unwrap()
        };

        match self.rasterizer() {
            SurfaceRasterizer::Rgb565(mut rast) => {
                font.draw_text(&mut rast, text, x as i32, y as i32 + font.baseline(), color);
            }
            SurfaceRasterizer::Luma4(mut rast) => {
                font.draw_text(&mut rast, text, x as i32, y as i32 + font.baseline(), color);
            }
        }
    }
}
