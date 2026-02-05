//! Info Demo App
//!
//! Simple UI demonstration with colored boxes and animated progress bar

use alloc::string::String;
use gfx::colors::Color;
use gfx::primitives::{CornerRadius, FillStyle, Rectangle};
use gfx::rasterizer::RasterTarget;
use micromath::F32Ext;
use swash::zeno::{Bounds, Point};

use crate::system::app_shell::{App, AppId};
use crate::system::input::{FocusEvent, InputEvent};
use crate::system::surface::Surface;

/// Demo app displaying a simple UI layout with animated elements
pub struct InfoDemo {
    name: String,
    frame_count: u32,
    service_timer: u32,
}

impl InfoDemo {
    pub fn new(name: String) -> Self {
        Self {
            name,
            frame_count: 0,
            service_timer: 0,
        }
    }
}

impl App for InfoDemo {
    fn init(&mut self, surface: &mut Surface) {
        surface.clear(Color::rgba(40, 40, 50, 255));
    }

    fn update(&mut self, surface: &mut Surface, _delta_ms: u32) {
        self.frame_count += 1;

        surface.clear(Color::rgba(40, 40, 50, 255));

        let mut rasterizer = surface.rasterizer();
        let width = rasterizer.width();
        let height = rasterizer.height();

        // Title bar
        Rectangle::new()
            .bounds(Bounds::new(
                Point::new(0.0, 0.0),
                Point::new(width as f32, 30.0),
            ))
            .fill(FillStyle::Solid(Color::rgba(70, 130, 180, 255)))
            .draw(&mut rasterizer);

        // Content area
        Rectangle::new()
            .bounds(Bounds::new(
                Point::new(10.0, 40.0),
                Point::new(width as f32 - 10.0, height as f32 - 10.0),
            ))
            .fill(FillStyle::Solid(Color::rgba(245, 245, 245, 255)))
            .corner_radii(CornerRadius::new(10.0, 10.0))
            .draw(&mut rasterizer);

        // Colored content boxes
        let colors = [
            Color::rgba(255, 100, 100, 255),
            Color::rgba(100, 255, 100, 255),
            Color::rgba(100, 100, 255, 255),
            Color::rgba(255, 255, 100, 255),
        ];

        for (i, color) in colors.iter().enumerate() {
            let y = 50.0 + (i as f32 * 25.0);
            Rectangle::new()
                .bounds(Bounds::new(
                    Point::new(20.0, y),
                    Point::new(120.0, y + 15.0),
                ))
                .fill(FillStyle::Solid(*color))
                .corner_radii(CornerRadius::new(3.0, 3.0))
                .draw(&mut rasterizer);
        }

        // Animated progress bar
        let progress =
            ((self.frame_count as f32 / 60.0).sin() * 0.5 + 0.5) * (width as f32 - 40.0);

        Rectangle::new()
            .bounds(Bounds::new(
                Point::new(20.0, height as f32 - 40.0),
                Point::new(20.0 + progress, height as f32 - 20.0),
            ))
            .fill(FillStyle::Solid(Color::rgba(70, 180, 130, 255)))
            .corner_radii(CornerRadius::new(10.0, 10.0))
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
            InputEvent::Button { id, pressed } => {
                if pressed {
                    defmt::info!("[{}] Button {} clicked!", self.name.as_str(), id);
                }
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
