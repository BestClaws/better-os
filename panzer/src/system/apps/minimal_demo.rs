//! Minimal Demo App
//!
//! Draws only a small rectangle without clearing - tests dirty region optimization

use alloc::string::String;
use gfx::colors::Color;

use crate::system::app_shell::{App, AppId};
use crate::system::input::{FocusEvent, InputEvent};
use crate::system::surface::Surface;

/// Minimal demo app that draws only a small rectangle
pub struct MinimalDemo {
    name: String,
    frame_count: u32,
}

impl MinimalDemo {
    pub fn new(name: String) -> Self {
        Self {
            name,
            frame_count: 0,
        }
    }
}

impl App for MinimalDemo {
    fn init(&mut self, surface: &mut Surface) {
        // Clear once on init
        surface.clear(Color::rgba(30, 30, 40, 255));
    }

    fn update(&mut self, surface: &mut Surface, _delta_ms: u32) {
        self.frame_count += 1;
        
        // Draw a small animated rectangle - NO CLEAR
        // This should result in minimal dirty region
        let x = 10 + ((self.frame_count / 2) % 80) as i16;
        let y = 50;
        
        // Draw small 20x20 rectangle
        surface.fill_rect(x, y, 20, 20, Color::rgba(255, 100, 100, 255));
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
