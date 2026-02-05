use crate::system::input::InputEvent;
use crate::system::surface::Surface;
use alloc::boxed::Box;
use alloc::vec::Vec;

/// Rectangle bounds for layout
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rect {
    pub x: i16,
    pub y: i16,
    pub width: u16,
    pub height: u16,
}

impl Rect {
    pub fn new(x: i16, y: i16, width: u16, height: u16) -> Self {
        Self { x, y, width, height }
    }

    pub fn contains(&self, x: i16, y: i16) -> bool {
        x >= self.x
            && y >= self.y
            && x < self.x + self.width as i16
            && y < self.y + self.height as i16
    }
}

/// Events that widgets can emit
#[derive(Debug, Clone)]
pub enum WidgetEvent {
    ButtonPressed,
    ButtonReleased,
    ValueChanged(i32),
    None,
}

/// Core widget trait
pub trait Widget: Send {
    /// Render the widget to a surface
    fn render(&self, surface: &mut Surface);

    /// Handle input events, return true if consumed
    fn handle_input(&mut self, event: &InputEvent) -> WidgetEvent;

    /// Set the layout bounds for this widget
    fn set_bounds(&mut self, bounds: Rect);

    /// Get current bounds
    fn bounds(&self) -> Rect;

    /// Check if point is inside widget
    fn contains_point(&self, x: i16, y: i16) -> bool {
        self.bounds().contains(x, y)
    }
}

/// Container that holds multiple child widgets
pub struct Container {
    bounds: Rect,
    children: Vec<Box<dyn Widget>>,
}

impl Container {
    pub fn new() -> Self {
        Self {
            bounds: Rect::new(0, 0, 0, 0),
            children: Vec::new(),
        }
    }

    pub fn add_child(&mut self, child: Box<dyn Widget>) {
        self.children.push(child);
    }
}

impl Widget for Container {
    fn render(&self, surface: &mut Surface) {
        for child in &self.children {
            child.render(surface);
        }
    }

    fn handle_input(&mut self, event: &InputEvent) -> WidgetEvent {
        // Pass input to children in reverse order (top to bottom)
        for child in self.children.iter_mut().rev() {
            let result = child.handle_input(event);
            if !matches!(result, WidgetEvent::None) {
                return result;
            }
        }
        WidgetEvent::None
    }

    fn set_bounds(&mut self, bounds: Rect) {
        self.bounds = bounds;
    }

    fn bounds(&self) -> Rect {
        self.bounds
    }
}

/// Vertical stack layout
pub struct VStack {
    bounds: Rect,
    children: Vec<Box<dyn Widget>>,
    spacing: u16,
}

impl VStack {
    pub fn new(spacing: u16) -> Self {
        Self {
            bounds: Rect::new(0, 0, 0, 0),
            children: Vec::new(),
            spacing,
        }
    }

    pub fn add_child(&mut self, child: Box<dyn Widget>) {
        self.children.push(child);
        self.layout_children();
    }

    fn layout_children(&mut self) {
        let mut y = self.bounds.y;
        let width = self.bounds.width;

        for child in &mut self.children {
            let child_height = child.bounds().height;
            child.set_bounds(Rect::new(self.bounds.x, y, width, child_height));
            y += child_height as i16 + self.spacing as i16;
        }
    }
}

impl Widget for VStack {
    fn render(&self, surface: &mut Surface) {
        for child in &self.children {
            child.render(surface);
        }
    }

    fn handle_input(&mut self, event: &InputEvent) -> WidgetEvent {
        for child in self.children.iter_mut().rev() {
            let result = child.handle_input(event);
            if !matches!(result, WidgetEvent::None) {
                return result;
            }
        }
        WidgetEvent::None
    }

    fn set_bounds(&mut self, bounds: Rect) {
        self.bounds = bounds;
        self.layout_children();
    }

    fn bounds(&self) -> Rect {
        self.bounds
    }
}

/// Horizontal stack layout
pub struct HStack {
    bounds: Rect,
    children: Vec<Box<dyn Widget>>,
    spacing: u16,
}

impl HStack {
    pub fn new(spacing: u16) -> Self {
        Self {
            bounds: Rect::new(0, 0, 0, 0),
            children: Vec::new(),
            spacing,
        }
    }

    pub fn add_child(&mut self, child: Box<dyn Widget>) {
        self.children.push(child);
        self.layout_children();
    }

    fn layout_children(&mut self) {
        let mut x = self.bounds.x;
        let height = self.bounds.height;

        for child in &mut self.children {
            let child_width = child.bounds().width;
            child.set_bounds(Rect::new(x, self.bounds.y, child_width, height));
            x += child_width as i16 + self.spacing as i16;
        }
    }
}

impl Widget for HStack {
    fn render(&self, surface: &mut Surface) {
        for child in &self.children {
            child.render(surface);
        }
    }

    fn handle_input(&mut self, event: &InputEvent) -> WidgetEvent {
        for child in self.children.iter_mut().rev() {
            let result = child.handle_input(event);
            if !matches!(result, WidgetEvent::None) {
                return result;
            }
        }
        WidgetEvent::None
    }

    fn set_bounds(&mut self, bounds: Rect) {
        self.bounds = bounds;
        self.layout_children();
    }

    fn bounds(&self) -> Rect {
        self.bounds
    }
}
