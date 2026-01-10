use crate::libs::gfx::core::color::{Color, ColorAlpha, Opacity};
use crate::libs::gfx::core::blend::{BlendDescriptor, BlendSource, BlendMode};
use crate::libs::gfx::core::geometry::{Point, Rect};
use crate::libs::gfx::draw_target::DrawTarget;
use crate::libs::gfx::layer::Layer;
use crate::libs::gfx::primitives::rectangle::Fill;

/// Arc primitive builder
pub struct Arc<'a> {
    layer: &'a mut Layer<'a>,
    center: Point,
    radius: u16,
    start_angle: i32,
    end_angle: i32,
    color: Color,
    width: i32,
    opacity: Opacity,
    rounded: bool,
}

impl<'a> Arc<'a> {
    pub fn new(layer: &'a mut Layer<'a>, center: Point, radius: u16) -> Self {
        Self {
            layer,
            center,
            radius,
            start_angle: 0,
            end_angle: 360,
            color: Color::WHITE,
            width: 2,
            opacity: Opacity::OPAQUE,
            rounded: false,
        }
    }

    pub fn angles(mut self, start: i32, end: i32) -> Self {
        self.start_angle = start;
        self.end_angle = end;
        self
    }

    pub fn start_angle(mut self, angle: i32) -> Self {
        self.start_angle = angle;
        self
    }

    pub fn end_angle(mut self, angle: i32) -> Self {
        self.end_angle = angle;
        self
    }

    pub fn color(mut self, color: Color) -> Self {
        self.color = color;
        self
    }

    pub fn width(mut self, width: i32) -> Self {
        self.width = width.max(1);
        self
    }

    pub fn opacity(mut self, opacity: Opacity) -> Self {
        self.opacity = opacity;
        self
    }

    pub fn rounded(mut self) -> Self {
        self.rounded = true;
        self
    }

    pub fn draw(mut self) {
        if self.width == 0 || self.radius == 0 {
            return;
        }

        // Full circle (360 degrees)
        if (self.start_angle + 360 == self.end_angle) || (self.start_angle == self.end_angle + 360) {
            self.draw_full_ring();
            return;
        }

        // Arc with angle
        self.draw_arc_segment();
    }

    fn draw_full_ring(mut self) {
        // Draw as a border (ring)
        let outer_rect = Rect::new(
            self.center.x - self.radius as i32,
            self.center.y - self.radius as i32,
            (self.radius as u32) * 2,
            (self.radius as u32) * 2
        );

        // For a full ring, we can use border primitive
        // This is simplified - proper implementation would use mask
        let inner_radius = (self.radius as i32 - self.width).max(0) as u16;
        
        // Draw outer circle
        self.draw_circle_outline(self.center, self.radius);
        
        // Draw inner circle (if radius is large enough)
        if inner_radius > 0 {
            // Clear inner area by drawing in background color
            // This is a simplification - proper implementation uses masks
        }
    }

    fn draw_arc_segment(mut self) {
        // Bresenham's circle algorithm adapted for arc segments
        let mut x = 0i32;
        let mut y = self.radius as i32;
        let mut d = 3 - 2 * self.radius as i32;

        // Normalize angles to 0-360
        let mut start = self.start_angle;
        let mut end = self.end_angle;
        while start < 0 { start += 360; }
        while end < 0 { end += 360; }
        while start >= 360 { start -= 360; }
        while end >= 360 { end -= 360; }

        while y >= x {
            // Draw 8 octants, but only if within angle range
            self.draw_arc_pixel(x, y, start, end);
            self.draw_arc_pixel(-x, y, start, end);
            self.draw_arc_pixel(x, -y, start, end);
            self.draw_arc_pixel(-x, -y, start, end);
            self.draw_arc_pixel(y, x, start, end);
            self.draw_arc_pixel(-y, x, start, end);
            self.draw_arc_pixel(y, -x, start, end);
            self.draw_arc_pixel(-y, -x, start, end);

            x += 1;
            if d > 0 {
                y -= 1;
                d = d + 4 * (x - y) + 10;
            } else {
                d = d + 4 * x + 6;
            }
        }
    }

    fn draw_arc_pixel(&mut self, x: i32, y: i32, start_angle: i32, end_angle: i32) {
        // Calculate angle of this pixel using integer approximation
        let mut angle_deg = if x == 0 && y == 0 {
            0
        } else if x >= 0 && y >= 0 {
            if x >= y { 45 * y / (x.max(1)) } else { 90 - 45 * x / (y.max(1)) }
        } else if x < 0 && y >= 0 {
            if -x >= y { 180 - 45 * y / ((-x).max(1)) } else { 90 + 45 * (-x) / (y.max(1)) }
        } else if x < 0 && y < 0 {
            if -x >= -y { 180 + 45 * (-y) / ((-x).max(1)) } else { 270 - 45 * (-x) / ((-y).max(1)) }
        } else {
            if x >= -y { 360 - 45 * (-y) / (x.max(1)) } else { 270 + 45 * x / ((-y).max(1)) }
        };

        // Check if angle is within arc range
        let in_range = if end_angle >= start_angle {
            angle_deg >= start_angle && angle_deg <= end_angle
        } else {
            angle_deg >= start_angle || angle_deg <= end_angle
        };

        if in_range {
            let px = self.center.x + x;
            let py = self.center.y + y;
            
            // Draw a small rectangle for line thickness
            let half_width = self.width / 2;
            let rect = Rect::new(
                px - half_width,
                py - half_width,
                self.width as u32,
                self.width as u32
            );
            
            let desc = BlendDescriptor {
                mode: BlendMode::Normal,
                source: Some(BlendSource::SolidColor { color: ColorAlpha::from_rgb(self.color, self.opacity) }),
                dest_rect: rect,
                opacity: self.opacity,
                mask: None,
            };
            self.layer.blend(&desc);
        }
    }

    fn draw_circle_outline(&mut self, center: Point, radius: u16) {
        // Bresenham's circle algorithm
        let mut x = 0i32;
        let mut y = radius as i32;
        let mut d = 3 - 2 * radius as i32;

        while y >= x {
            // Draw 8 octants
            self.draw_circle_pixels(center, x, y);
            x += 1;
            if d > 0 {
                y -= 1;
                d = d + 4 * (x - y) + 10;
            } else {
                d = d + 4 * x + 6;
            }
        }
    }

    fn draw_circle_pixels(&mut self, center: Point, x: i32, y: i32) {
        let points = [
            (center.x + x, center.y + y),
            (center.x - x, center.y + y),
            (center.x + x, center.y - y),
            (center.x - x, center.y - y),
            (center.x + y, center.y + x),
            (center.x - y, center.y + x),
            (center.x + y, center.y - x),
            (center.x - y, center.y - x),
        ];

        for (px, py) in points.iter() {
            let half_width = self.width / 2;
            let rect = Rect::new(
                px - half_width,
                py - half_width,
                self.width as u32,
                self.width as u32
            );
            
            let desc = BlendDescriptor {
                mode: BlendMode::Normal,
                source: Some(BlendSource::SolidColor { color: ColorAlpha::from_rgb(self.color, self.opacity) }),
                dest_rect: rect,
                opacity: self.opacity,
                mask: None,
            };
            self.layer.blend(&desc);
        }
    }
}

/// Circle primitive (filled circle)
pub struct Circle<'a> {
    layer: &'a mut Layer<'a>,
    center: Point,
    radius: u16,
    color: Color,
    opacity: Opacity,
}

impl<'a> Circle<'a> {
    pub fn new(layer: &'a mut Layer<'a>, center: Point, radius: u16) -> Self {
        Self {
            layer,
            center,
            radius,
            color: Color::WHITE,
            opacity: Opacity::OPAQUE,
        }
    }

    pub fn color(mut self, color: Color) -> Self {
        self.color = color;
        self
    }

    pub fn opacity(mut self, opacity: Opacity) -> Self {
        self.opacity = opacity;
        self
    }

    pub fn draw(mut self) {
        if self.radius == 0 {
            return;
        }

        // Use midpoint circle algorithm to fill the circle
        let mut x = 0i32;
        let mut y = self.radius as i32;
        let mut d = 3 - 2 * self.radius as i32;

        while y >= x {
            // Draw horizontal lines to fill the circle
            self.draw_horizontal_line(self.center.y + y, self.center.x - x, self.center.x + x);
            self.draw_horizontal_line(self.center.y - y, self.center.x - x, self.center.x + x);
            self.draw_horizontal_line(self.center.y + x, self.center.x - y, self.center.x + y);
            self.draw_horizontal_line(self.center.y - x, self.center.x - y, self.center.x + y);

            x += 1;
            if d > 0 {
                y -= 1;
                d = d + 4 * (x - y) + 10;
            } else {
                d = d + 4 * x + 6;
            }
        }
    }

    fn draw_horizontal_line(&mut self, y: i32, x1: i32, x2: i32) {
        if x2 < x1 {
            return;
        }
        
        let rect = Rect::new(x1, y, (x2 - x1 + 1) as u32, 1);
        let desc = BlendDescriptor {
            mode: BlendMode::Normal,
            source: Some(BlendSource::SolidColor { color: ColorAlpha::from_rgb(self.color, self.opacity) }),
            dest_rect: rect,
            opacity: self.opacity,
            mask: None,
        };
        self.layer.blend(&desc);
    }
}

impl<'a> Layer<'a> {
    pub fn arc(&'a mut self, center: Point, radius: u16) -> Arc<'a> {
        Arc::new(self, center, radius)
    }

    pub fn circle(&'a mut self, center: Point, radius: u16) -> Circle<'a> {
        Circle::new(self, center, radius)
    }
}
