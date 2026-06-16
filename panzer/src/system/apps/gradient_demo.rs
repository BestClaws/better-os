//! Gradient Demo App
//!
//! Animated color gradient demonstration

use alloc::string::String;
use gfx::colors::Color;
use gfx::primitives::{CornerRadius, FillStyle, Rectangle};
use gfx::rasterizer::RasterTarget;
use swash::zeno::{Bounds, Point};

use crate::system::app_shell::{App, AppId};
use crate::system::input::{FocusEvent, InputEvent};
use crate::system::surface::Surface;

/// Demo app that displays an animated color gradient
pub struct GradientDemo {
    name: String,
    offset: f32,
    service_timer: u32,
}

impl GradientDemo {
    pub fn new(name: String) -> Self {
        Self {
            name,
            offset: 0.0,
            service_timer: 0,
        }
    }
}

impl App for GradientDemo {
    fn init(&mut self, surface: &mut Surface) {
        surface.clear(Color::rgba(0, 0, 0, 255));
    }

    fn update(&mut self, surface: &mut Surface, delta_ms: u32) {
        self.offset += delta_ms as f32 / 50.0;

        let mut rasterizer = surface.rasterizer();
        let width = rasterizer.width();
        let height = rasterizer.height();

        // Draw animated horizontal gradient strips
        for y in 0..height {
            let t = (y as f32 + self.offset) / height as f32;
            let t = t % 1.0;

            let r = (t * 255.0) as u8;
            let g = ((1.0 - t) * 255.0) as u8;
            let b = ((t * 0.5 + 0.5) * 255.0) as u8;

            let color = Color::rgba(r, g, b, 255);
            rasterizer.fill_solid_hspan(y, 0, color, width);
        }

        // Overlay semi-transparent rounded rectangle
        let rect_color = Color::rgba(255, 255, 255, 128);
        let rx = width as f32 / 4.0;
        let ry = height as f32 / 4.0;
        let rw = width as f32 / 2.0;
        let rh = height as f32 / 2.0;

        Rectangle::new()
            .bounds(Bounds::new(
                Point::new(rx, ry),
                Point::new(rx + rw, ry + rh),
            ))
            .fill(FillStyle::Solid(rect_color))
            .corner_radii(CornerRadius::new(20.0, 20.0))
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
            _ => false,
        }
    }

    fn on_focus(&mut self, event: FocusEvent) {
        match event {
            FocusEvent::Gained => defmt::info!("[{}] Focus gained", self.name.as_str()),
            FocusEvent::Lost => defmt::info!("[{}] Focus lost", self.name.as_str()),
        }
    }

    fn on_message(&mut self, from: AppId, data: &[u8]) {
        if let Ok(text) = core::str::from_utf8(data) {
            defmt::info!("[{}] Message from {:?}: {}", self.name.as_str(), from, text);
        }
    }

    fn name(&self) -> &str {
        &self.name
    }
}
