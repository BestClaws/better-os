use crate::system::app_shell::{App, AppId, Message};
use crate::system::input::InputEvent;
use crate::system::surface::Surface;
use crate::system::ui::{Button, Label, TextAlign, VStack, Widget, WidgetEvent, Rect};
use alloc::boxed::Box;
use alloc::string::{String, ToString};
use defmt::info;
use gfx::colors::Color;

pub struct WidgetDemo {
    app_id: AppId,
    root: VStack,
    button_count: u32,
}

impl WidgetDemo {
    pub fn new(app_id: AppId) -> Self {
        // Create UI hierarchy
        let mut root = VStack::new(10);
        root.set_bounds(Rect::new(10, 30, 185, 200));

        // Title label
        let title = Label::new("Widget Demo".to_string())
            .with_color(Color::rgba(255, 255, 0, 255))
            .with_align(TextAlign::Center)
            .with_size(185, 30);

        // Button 1
        let button1 = Button::new("Click Me!".to_string())
            .with_colors(
                Color::rgba(40, 80, 120, 255),
                Color::rgba(255, 255, 255, 255),
                Color::rgba(80, 120, 160, 255),
            )
            .with_size(185, 40);

        // Button 2
        let button2 = Button::new("Press Here".to_string())
            .with_colors(
                Color::rgba(120, 40, 40, 255),
                Color::rgba(255, 255, 255, 255),
                Color::rgba(160, 80, 80, 255),
            )
            .with_size(185, 40);

        // Counter label
        let counter = Label::new("Count: 0".to_string())
            .with_color(Color::rgba(200, 200, 200, 255))
            .with_align(TextAlign::Center)
            .with_size(185, 25);

        root.add_child(Box::new(title));
        root.add_child(Box::new(button1));
        root.add_child(Box::new(button2));
        root.add_child(Box::new(counter));

        Self {
            app_id,
            root,
            button_count: 0,
        }
    }
}

impl App for WidgetDemo {
    fn id(&self) -> AppId {
        self.app_id
    }

    fn name(&self) -> &str {
        "Widget Demo"
    }

    fn update(&mut self, _delta_ms: u32) {}

    fn render(&mut self, surface: &mut Surface) {
        // Clear background
        surface.clear(Color::rgba(20, 20, 20, 255));

        // Render widget tree
        self.root.render(surface);
    }

    fn on_input(&mut self, event: InputEvent) {
        info!("[WidgetDemo] Input: {:?}", event);

        // Pass input to widget tree
        let result = self.root.handle_input(&event);

        match result {
            WidgetEvent::ButtonPressed => {
                info!("[WidgetDemo] Button pressed!");
            }
            WidgetEvent::ButtonReleased => {
                self.button_count += 1;
                info!("[WidgetDemo] Button released! Count: {}", self.button_count);
                
                // Update counter label (would need widget tree traversal or references)
                // For now, just log it
            }
            _ => {}
        }
    }

    fn on_message(&mut self, from: AppId, data: &[u8]) {
        info!(
            "[WidgetDemo] Message from {:?}: {} bytes",
            from,
            data.len()
        );
    }

    fn on_focus(&mut self, gained: bool) {
        if gained {
            info!("[WidgetDemo] Focus gained");
        } else {
            info!("[WidgetDemo] Focus lost");
        }
    }

    fn on_lifecycle(&mut self, event: crate::system::app_shell::LifecycleEvent) {
        info!("[WidgetDemo] Lifecycle: {:?}", event);
    }
}
