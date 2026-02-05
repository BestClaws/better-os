use super::widget::{Rect, Widget, WidgetEvent};
use crate::system::input::InputEvent;
use crate::system::surface::{Surface, DisplayInfo};
use alloc::string::String;
use gfx::colors::Color;

#[derive(Debug, Clone, Copy)]
pub enum TextAlign {
    Left,
    Center,
    Right,
}

pub struct Label {
    bounds: Rect,
    text: String,
    color: Color,
    align: TextAlign,
    // Store logical size
    logical_width: f32,
    logical_height: f32,
}

impl Label {
    pub fn new(text: String, display_info: DisplayInfo) -> Self {
        // Default logical size
        let logical_width = 200.0;
        let logical_height = 20.0;
        
        let width = display_info.scale(logical_width) as u16;
        let height = display_info.scale(logical_height) as u16;
        
        Self {
            bounds: Rect::new(0, 0, width, height),
            text,
            color: Color::rgba(255, 255, 255, 255),
            align: TextAlign::Left,
            logical_width,
            logical_height,
        }
    }

    pub fn with_color(mut self, color: Color) -> Self {
        self.color = color;
        self
    }

    pub fn with_align(mut self, align: TextAlign) -> Self {
        self.align = align;
        self
    }

    pub fn set_text(&mut self, text: String) {
        self.text = text;
    }

    pub fn with_logical_size(mut self, display_info: DisplayInfo, width: f32, height: f32) -> Self {
        self.logical_width = width;
        self.logical_height = height;
        self.bounds.width = display_info.scale(width) as u16;
        self.bounds.height = display_info.scale(height) as u16;
        self
    }
}

impl Widget for Label {
    fn render(&self, surface: &mut Surface) {
        let text_width = self.text.len() as u16 * 8; // Approximate

        let x = match self.align {
            TextAlign::Left => self.bounds.x,
            TextAlign::Center => {
                self.bounds.x + (self.bounds.width as i16 - text_width as i16) / 2
            }
            TextAlign::Right => self.bounds.x + self.bounds.width as i16 - text_width as i16,
        };

        let y = self.bounds.y + (self.bounds.height as i16 - 16) / 2;

        surface.draw_text(x, y, &self.text, self.color);
    }

    fn handle_input(&mut self, _event: &InputEvent) -> WidgetEvent {
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
