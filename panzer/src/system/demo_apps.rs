//! Demo applications
//!
//! Example applications that draw to surfaces

use alloc::string::{String, ToString};
use gfx::colors::Color;
use gfx::primitives::{CornerRadius, Edge, FillStyle, Rectangle, StrokeStyle};
use gfx::rasterizer::RasterTarget;
use micromath::F32Ext;
use swash::zeno::{Bounds, Point};

use crate::system::app_shell::App;
use crate::system::surface::Surface;
use crate::system::input::{InputEvent, FocusEvent, LifecycleEvent};
use crate::system::app_shell::AppId;

/// A simple demo app that draws animated shapes
pub struct ShapesDemo {
    name: String,
    time: f32,
    service_timer: u32,
}

impl ShapesDemo {
    pub fn new(name: String) -> Self {
        Self { name, time: 0.0, service_timer: 0 }
    }
}

impl App for ShapesDemo {
    fn init(&mut self, surface: &mut Surface) {
        // Clear with background color
        surface.clear(Color::rgba(20, 20, 30, 255));
    }

    fn update(&mut self, surface: &mut Surface, delta_ms: u32) {
        self.time += delta_ms as f32 / 1000.0;

        // Clear background
        surface.clear(Color::rgba(20, 20, 30, 255));

        // Get display info for resolution-independent sizing
        let display_info = surface.display_info();
        let scale = |px: f32| display_info.scale(px);

        let mut rasterizer = surface.rasterizer();
        let width = rasterizer.width();
        let height = rasterizer.height();

        // Animated bouncing rectangle - sizes scale with DPI
        let rect_size = scale(40.0);
        let x = ((self.time * 60.0).sin() * 0.4 + 0.5) * (width as f32 - rect_size);
        let y = ((self.time * 40.0).cos() * 0.4 + 0.5) * (height as f32 - rect_size);

        let rect_color = Color::rgba(255, 100, 100, 255);
        Rectangle::new()
            .bounds(Bounds::new(Point::new(x, y), Point::new(x + rect_size, y + rect_size)))
            .fill(FillStyle::Solid(rect_color))
            .corner_radii(CornerRadius::new(8.0, 8.0))
            .draw(&mut rasterizer);

        // Rotating square in center - size scales with DPI
        let center_x = width as f32 / 2.0;
        let center_y = height as f32 / 2.0;
        let size = scale(50.0);
        
        let angle = self.time;
        let cos_a = angle.cos();
        let sin_a = angle.sin();
        
        // Draw a simple cross pattern
        let line_color = Color::rgba(100, 255, 100, 255);
        let half_size = size / 2.0;
        
        // Horizontal line
        Rectangle::new()
            .bounds(Bounds::new(
                Point::new(center_x - half_size, center_y - 2.0),
                Point::new(center_x + half_size, center_y + 2.0),
            ))
            .fill(FillStyle::Solid(line_color))
            .draw(&mut rasterizer);
        
        // Vertical line
        Rectangle::new()
            .bounds(Bounds::new(
                Point::new(center_x - 2.0, center_y - half_size),
                Point::new(center_x + 2.0, center_y + half_size),
            ))
            .fill(FillStyle::Solid(line_color))
            .draw(&mut rasterizer);

        // Border rectangles - width scales with DPI
        let border_color = Color::rgba(100, 100, 255, 255);
        let border_width = scale(3.0);
        let margin = scale(10.0);
        
        // Top border
        Rectangle::new()
            .bounds(Bounds::new(Point::new(margin, margin), Point::new(width as f32 - margin, margin + border_width)))
            .fill(FillStyle::Solid(border_color))
            .draw(&mut rasterizer);
        
        // Bottom border
        Rectangle::new()
            .bounds(Bounds::new(Point::new(margin, height as f32 - margin - border_width), Point::new(width as f32 - margin, height as f32 - margin)))
            .fill(FillStyle::Solid(border_color))
            .draw(&mut rasterizer);
        
        // Left border
        Rectangle::new()
            .bounds(Bounds::new(Point::new(margin, margin), Point::new(margin + border_width, height as f32 - margin)))
            .fill(FillStyle::Solid(border_color))
            .draw(&mut rasterizer);
        
        // Right border
        Rectangle::new()
            .bounds(Bounds::new(Point::new(width as f32 - margin - border_width, margin), Point::new(width as f32 - margin, height as f32 - margin)))
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
                defmt::info!("[{}] Touch: ({}, {}) pressed={}", self.name.as_str(), x, y, pressed);
                true
            }
            InputEvent::Button { id, pressed } => {
                defmt::info!("[{}] Button {} {}", self.name.as_str(), id, if pressed { "pressed" } else { "released" });
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
            defmt::info!("[{}] Binary message from {:?} ({} bytes)", self.name.as_str(), from, data.len());
        }
    }

    fn name(&self) -> &str {
        &self.name
    }
}

/// A color gradient demo
pub struct GradientDemo {
    name: String,
    offset: f32,
    service_timer: u32,
}

impl GradientDemo {
    pub fn new(name: String) -> Self {
        Self { name, offset: 0.0, service_timer: 0 }
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

        // Draw horizontal gradient strips
        for y in 0..height {
            let t = (y as f32 + self.offset) / height as f32;
            let t = (t % 1.0);
            
            let r = (t * 255.0) as u8;
            let g = ((1.0 - t) * 255.0) as u8;
            let b = ((t * 0.5 + 0.5) * 255.0) as u8;
            
            let color = Color::rgba(r, g, b, 255);
            rasterizer.fill_solid_hspan(y, 0, color, width);
        }

        // Draw some overlaid rectangles
        let rect_color = Color::rgba(255, 255, 255, 128);
        let rx = width as f32 / 4.0;
        let ry = height as f32 / 4.0;
        let rw = width as f32 / 2.0;
        let rh = height as f32 / 2.0;
        Rectangle::new()
            .bounds(Bounds::new(Point::new(rx, ry), Point::new(rx + rw, ry + rh)))
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
                defmt::info!("[{}] Touch: ({}, {}) pressed={}", self.name.as_str(), x, y, pressed);
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

/// A simple text/info demo
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

    fn update(&mut self, surface: &mut Surface, delta_ms: u32) {
        self.frame_count += 1;

        // Clear with dark blue background
        surface.clear(Color::rgba(40, 40, 50, 255));

        let mut rasterizer = surface.rasterizer();
        let width = rasterizer.width();
        let height = rasterizer.height();

        // Draw title bar
        let title_color = Color::rgba(70, 130, 180, 255);
        Rectangle::new()
            .bounds(Bounds::new(Point::new(0.0, 0.0), Point::new(width as f32, 30.0)))
            .fill(FillStyle::Solid(title_color))
            .draw(&mut rasterizer);

        // Draw content area
        let content_color = Color::rgba(245, 245, 245, 255);
        Rectangle::new()
            .bounds(Bounds::new(Point::new(10.0, 40.0), Point::new(width as f32 - 10.0, height as f32 - 10.0)))
            .fill(FillStyle::Solid(content_color))
            .corner_radii(CornerRadius::new(10.0, 10.0))
            .draw(&mut rasterizer);

        // Draw some colored boxes as "text" placeholders
        let colors = [
            Color::rgba(255, 100, 100, 255),
            Color::rgba(100, 255, 100, 255),
            Color::rgba(100, 100, 255, 255),
            Color::rgba(255, 255, 100, 255),
        ];

        for (i, color) in colors.iter().enumerate() {
            let y = 50.0 + (i as f32 * 25.0);
            Rectangle::new()
                .bounds(Bounds::new(Point::new(20.0, y), Point::new(120.0, y + 15.0)))
                .fill(FillStyle::Solid(*color))
                .corner_radii(CornerRadius::new(3.0, 3.0))
                .draw(&mut rasterizer);
        }

        // Animated progress bar
        let progress = ((self.frame_count as f32 / 60.0).sin() * 0.5 + 0.5) * (width as f32 - 40.0);
        let progress_color = Color::rgba(70, 180, 130, 255);
        Rectangle::new()
            .bounds(Bounds::new(Point::new(20.0, height as f32 - 40.0), Point::new(20.0 + progress, height as f32 - 20.0)))
            .fill(FillStyle::Solid(progress_color))
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


/// Widget-based demo app
pub struct WidgetDemo {
    name: String,
    app_id: AppId,
    root: crate::system::ui::VStack,
    button_count: u32,
}

impl WidgetDemo {
    pub fn new(name: String, app_id: AppId) -> Self {
        use crate::system::ui::{Button, Label, TextAlign, VStack, Rect, Widget};
        use alloc::boxed::Box;

        let mut root = VStack::new(10);
        // Set bounds BEFORE adding children so layout works correctly
        root.set_bounds(Rect::new(10, 30, 185, 200));

        let title = Label::new("Widget Demo".to_string())
            .with_color(Color::rgba(255, 255, 0, 255))
            .with_align(TextAlign::Center)
            .with_size(185, 30);

        let button1 = Button::new("Click Me!".to_string())
            .with_colors(
                Color::rgba(40, 80, 120, 255),
                Color::rgba(255, 255, 255, 255),
                Color::rgba(80, 120, 160, 255),
            )
            .with_size(185, 40);

        let button2 = Button::new("Press Here".to_string())
            .with_colors(
                Color::rgba(120, 40, 40, 255),
                Color::rgba(255, 255, 255, 255),
                Color::rgba(160, 80, 80, 255),
            )
            .with_size(185, 40);

        let counter = Label::new("Count: 0".to_string())
            .with_color(Color::rgba(200, 200, 200, 255))
            .with_align(TextAlign::Center)
            .with_size(185, 25);

        root.add_child(Box::new(title));
        root.add_child(Box::new(button1));
        root.add_child(Box::new(button2));
        root.add_child(Box::new(counter));

        Self { name, app_id, root, button_count: 0 }
    }
}

impl App for WidgetDemo {
    fn init(&mut self, surface: &mut Surface) {
        surface.clear(Color::rgba(20, 20, 20, 255));
    }

    fn update(&mut self, surface: &mut Surface, _delta_ms: u32) {
        surface.clear(Color::rgba(20, 20, 20, 255));
        use crate::system::ui::Widget;
        self.root.render(surface);
    }

    fn on_input(&mut self, event: InputEvent) -> bool {
        use crate::system::ui::{Widget, WidgetEvent};
        let result = self.root.handle_input(&event);
        match result {
            WidgetEvent::ButtonPressed => {
                defmt::info!("[{}] Button pressed!", self.name.as_str());
                true
            }
            WidgetEvent::ButtonReleased => {
                self.button_count += 1;
                defmt::info!("[{}] Button #{}!", self.name.as_str(), self.button_count);
                true
            }
            _ => false
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
