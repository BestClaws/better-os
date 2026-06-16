//! Widget Demo App
//!
//! Interactive widget framework demonstration with buttons and labels

use alloc::boxed::Box;
use alloc::format;
use alloc::string::{String, ToString};
use gfx::colors::Color;

use crate::system::app_shell::{App, AppId};
use crate::system::input::{FocusEvent, InputEvent, LifecycleEvent};
use crate::system::surface::{DisplayInfo, Surface};
use crate::system::ui::{Button, Label, Rect, TextAlign, VStack, Widget, WidgetEvent};

/// Demo app showcasing the UI widget framework
pub struct WidgetDemo {
    name: String,
    app_id: AppId,
    root: VStack,
    button_count: u32,
    display_info: DisplayInfo,
}

impl WidgetDemo {
    pub fn new(name: String, app_id: AppId, display_info: DisplayInfo) -> Self {
        let mut root = VStack::new_with_dpi(10.0, display_info);

        // Define layout in logical coordinates (baseline 160 DPI)
        let logical_x = 10.0;
        let logical_y = 30.0;
        let logical_width = 185.0;
        let logical_height = 200.0;

        // Scale to physical pixels based on actual DPI
        let x = display_info.scale(logical_x) as i16;
        let y = display_info.scale(logical_y) as i16;
        let width = display_info.scale(logical_width) as u16;
        let height = display_info.scale(logical_height) as u16;

        root.set_bounds(Rect::new(x, y, width, height));

        // Create UI elements with DPI-aware sizing
        let title = Label::new("Widget Demo".to_string(), display_info)
            .with_color(Color::rgba(255, 255, 0, 255))
            .with_align(TextAlign::Center)
            .with_logical_size(display_info, 185.0, 30.0);

        let button1 = Button::new("Click Me!".to_string(), display_info)
            .with_colors(
                Color::rgba(40, 80, 120, 255),
                Color::rgba(255, 255, 255, 255),
                Color::rgba(80, 120, 160, 255),
            )
            .with_logical_size(display_info, 185.0, 40.0);

        let button2 = Button::new("Press Here".to_string(), display_info)
            .with_colors(
                Color::rgba(120, 40, 40, 255),
                Color::rgba(255, 255, 255, 255),
                Color::rgba(160, 80, 80, 255),
            )
            .with_logical_size(display_info, 185.0, 40.0);

        let counter = Label::new("Count: 0".to_string(), display_info)
            .with_color(Color::rgba(200, 200, 200, 255))
            .with_align(TextAlign::Center)
            .with_logical_size(display_info, 185.0, 25.0);

        root.add_child(Box::new(title));
        root.add_child(Box::new(button1));
        root.add_child(Box::new(button2));
        root.add_child(Box::new(counter));

        Self {
            name,
            app_id,
            root,
            button_count: 0,
            display_info,
        }
    }
}

impl App for WidgetDemo {
    fn init(&mut self, surface: &mut Surface) {
        surface.clear(Color::rgba(20, 20, 20, 255));
    }

    fn update(&mut self, surface: &mut Surface, _delta_ms: u32) {
        surface.clear(Color::rgba(20, 20, 20, 255));
        self.root.render(surface);
    }

    fn on_input(&mut self, event: InputEvent) -> bool {
        let result = self.root.handle_input(&event);
        match result {
            WidgetEvent::ButtonPressed => {
                defmt::info!("[{}] Button pressed!", self.name.as_str());
                true
            }
            WidgetEvent::ButtonReleased => {
                self.button_count += 1;
                defmt::info!("[{}] Button #{}!", self.name.as_str(), self.button_count);

                // Update counter label (4th child at index 3)
                if let Some(counter_widget) = self.root.child_mut(3) {
                    if let Some(label) = counter_widget.as_any_mut().downcast_mut::<Label>() {
                        label.set_text(format!("Count: {}", self.button_count));
                    }
                }
                true
            }
            _ => false,
        }
    }

    fn on_lifecycle(&mut self, event: LifecycleEvent) {
        defmt::info!("[{}] Lifecycle: {:?}", self.name.as_str(), event);
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
