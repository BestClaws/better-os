use super::widget::{Rect, Widget, WidgetEvent};
use crate::system::input::InputEvent;
use crate::system::surface::Surface;
use alloc::string::String;
use gfx::colors::Color;

pub struct Button {
    bounds: Rect,
    label: String,
    is_pressed: bool,
    bg_color: Color,
    fg_color: Color,
    pressed_color: Color,
}

impl Button {
    pub fn new(label: String) -> Self {
        Self {
            bounds: Rect::new(0, 0, 100, 40),
            label,
            is_pressed: false,
            bg_color: Color::rgba(60, 60, 60, 255),
            fg_color: Color::rgba(255, 255, 255, 255),
            pressed_color: Color::rgba(100, 100, 100, 255),
        }
    }

    pub fn with_colors(mut self, bg: Color, fg: Color, pressed: Color) -> Self {
        self.bg_color = bg;
        self.fg_color = fg;
        self.pressed_color = pressed;
        self
    }

    pub fn with_size(mut self, width: u16, height: u16) -> Self {
        self.bounds.width = width;
        self.bounds.height = height;
        self
    }
}

impl Widget for Button {
    fn render(&self, surface: &mut Surface) {
        let color = if self.is_pressed {
            self.pressed_color
        } else {
            self.bg_color
        };

        // Draw button background
        surface.fill_rect(
            self.bounds.x,
            self.bounds.y,
            self.bounds.width,
            self.bounds.height,
            color,
        );

        // Draw border
        surface.draw_rect(
            self.bounds.x,
            self.bounds.y,
            self.bounds.width,
            self.bounds.height,
            self.fg_color,
        );

        // Draw label centered
        let text_width = self.label.len() as u16 * 8; // Approximate
        let text_x = self.bounds.x + (self.bounds.width as i16 - text_width as i16) / 2;
        let text_y = self.bounds.y + (self.bounds.height as i16 - 16) / 2;

        surface.draw_text(text_x, text_y, &self.label, self.fg_color);
    }

    fn handle_input(&mut self, event: &InputEvent) -> WidgetEvent {
        match event {
            InputEvent::Touch { x, y, pressed } => {
                if self.contains_point(*x as i16, *y as i16) {
                    if *pressed && !self.is_pressed {
                        self.is_pressed = true;
                        return WidgetEvent::ButtonPressed;
                    } else if !*pressed && self.is_pressed {
                        self.is_pressed = false;
                        return WidgetEvent::ButtonReleased;
                    }
                } else if !*pressed {
                    self.is_pressed = false;
                }
            }
            _ => {}
        }
        WidgetEvent::None
    }

    fn set_bounds(&mut self, bounds: Rect) {
        self.bounds = bounds;
    }

    fn bounds(&self) -> Rect {
        self.bounds
    }

    fn as_any_mut(&mut self) -> &mut dyn core::any::Any {
        self
    }
}
