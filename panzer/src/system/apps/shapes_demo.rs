//! Shapes Demo App
//!
//! Animated shapes demonstration with DPI-aware scaling

use alloc::string::String;
use gfx::colors::Color;
use gfx::primitives::{CornerRadius, FillStyle, Rectangle};
use gfx::rasterizer::RasterTarget;
use micromath::F32Ext;
use swash::zeno::{Bounds, Point};

use crate::system::app_shell::{App, AppId};
use crate::system::input::{FocusEvent, InputEvent, LifecycleEvent};
use crate::system::surface::Surface;

/// Demo app that draws animated geometric shapes
pub struct ShapesDemo {
    name: String,
    time: f32,
    service_timer: u32,
}

impl ShapesDemo {
    pub fn new(name: String) -> Self {
        Self {
            name,
            time: 0.0,
            service_timer: 0,
        }
    }
}

impl App for ShapesDemo {
    fn init(&mut self, surface: &mut Surface) {
        surface.clear(Color::rgba(20, 20, 30, 255));
    }

    fn update(&mut self, surface: &mut Surface, delta_ms: u32) {
        self.time += delta_ms as f32 / 1000.0;

        surface.clear(Color::rgba(20, 20, 30, 255));

        // Get display info for resolution-independent sizing
        let display_info = surface.display_info();
        let scale = |px: f32| display_info.scale(px);

        let mut rasterizer = surface.rasterizer();
        let width = rasterizer.width();
        let height = rasterizer.height();

        // Animated bouncing rectangle
        let rect_size = scale(40.0);
        let x = ((self.time * 60.0).sin() * 0.4 + 0.5) * (width as f32 - rect_size);
        let y = ((self.time * 40.0).cos() * 0.4 + 0.5) * (height as f32 - rect_size);

        Rectangle::new()
            .bounds(Bounds::new(
                Point::new(x, y),
                Point::new(x + rect_size, y + rect_size),
            ))
            .fill(FillStyle::Solid(Color::rgba(255, 100, 100, 255)))
            .corner_radii(CornerRadius::new(8.0, 8.0))
            .draw(&mut rasterizer);

        // Rotating cross in center
        let center_x = width as f32 / 2.0;
        let center_y = height as f32 / 2.0;
        let size = scale(50.0);
        let half_size = size / 2.0;

        // Horizontal line
        Rectangle::new()
            .bounds(Bounds::new(
                Point::new(center_x - half_size, center_y - 2.0),
                Point::new(center_x + half_size, center_y + 2.0),
            ))
            .fill(FillStyle::Solid(Color::rgba(100, 255, 100, 255)))
            .draw(&mut rasterizer);

        // Vertical line
        Rectangle::new()
            .bounds(Bounds::new(
                Point::new(center_x - 2.0, center_y - half_size),
                Point::new(center_x + 2.0, center_y + half_size),
            ))
            .fill(FillStyle::Solid(Color::rgba(100, 255, 100, 255)))
            .draw(&mut rasterizer);

        // Border frame
        let border_color = Color::rgba(100, 100, 255, 255);
        let border_width = scale(3.0);
        let margin = scale(10.0);

        // Top
        Rectangle::new()
            .bounds(Bounds::new(
                Point::new(margin, margin),
                Point::new(width as f32 - margin, margin + border_width),
            ))
            .fill(FillStyle::Solid(border_color))
            .draw(&mut rasterizer);

        // Bottom
        Rectangle::new()
            .bounds(Bounds::new(
                Point::new(margin, height as f32 - margin - border_width),
                Point::new(width as f32 - margin, height as f32 - margin),
            ))
            .fill(FillStyle::Solid(border_color))
            .draw(&mut rasterizer);

        // Left
        Rectangle::new()
            .bounds(Bounds::new(
                Point::new(margin, margin),
                Point::new(margin + border_width, height as f32 - margin),
            ))
            .fill(FillStyle::Solid(border_color))
            .draw(&mut rasterizer);

        // Right
        Rectangle::new()
            .bounds(Bounds::new(
                Point::new(width as f32 - margin - border_width, margin),
                Point::new(width as f32 - margin, height as f32 - margin),
            ))
            .fill(FillStyle::Solid(border_color))
            .draw(&mut rasterizer);
    }

    fn service_update(&mut self, delta_ms: u32) {
        self.service_timer += delta_ms;
        if self.service_timer >= 1000 {
            defmt::info!("[Service] {}", self.name.as_str());
            self.service_timer = 0;
        }
    }

    fn on_input(&mut self, event: InputEvent) -> bool {
        match event {
            InputEvent::Touch { x, y, pressed } => {
                defmt::info!(
                    "[{}] Touch: ({}, {}) pressed={}",
                    self.name.as_str(),
                    x,
                    y,
                    pressed
                );
                true
            }
            InputEvent::Button { id, pressed } => {
                defmt::info!(
                    "[{}] Button {} {}",
                    self.name.as_str(),
                    id,
                    if pressed { "pressed" } else { "released" }
                );
                true
            }
            InputEvent::Keyboard { key } => {
                defmt::info!("[{}] Key: {}", self.name.as_str(), key);
                true
            }
        }
    }

    fn on_focus(&mut self, event: FocusEvent) {
        match event {
            FocusEvent::Gained => defmt::info!("[{}] Focus gained", self.name.as_str()),
            FocusEvent::Lost => defmt::info!("[{}] Focus lost", self.name.as_str()),
        }
    }

    fn on_lifecycle(&mut self, event: LifecycleEvent) {
        defmt::info!("[{}] Lifecycle: {:?}", self.name.as_str(), event);
    }

    fn on_message(&mut self, from: AppId, data: &[u8]) {
        if let Ok(text) = core::str::from_utf8(data) {
            defmt::info!("[{}] Message from {:?}: {}", self.name.as_str(), from, text);
        } else {
            defmt::info!(
                "[{}] Binary message from {:?} ({} bytes)",
                self.name.as_str(),
                from,
                data.len()
            );
        }
    }

    fn name(&self) -> &str {
        &self.name
    }
}
