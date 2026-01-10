use crate::libs::gfx::core::color::{Color, ColorAlpha, Opacity};
use crate::libs::gfx::core::blend::{BlendDescriptor, BlendSource, BlendMode};
use crate::libs::gfx::core::geometry::{PointF, Rect};
use crate::libs::gfx::draw_target::DrawTarget;
use crate::libs::gfx::layer::Layer;

/// Line primitive builder
pub struct Line<'a> {
    layer: &'a mut Layer<'a>,
    p1: PointF,
    p2: PointF,
    color: Color,
    width: i32,
    opacity: Opacity,
    dash_width: i32,
    dash_gap: i32,
    round_start: bool,
    round_end: bool,
}

impl<'a> Line<'a> {
    pub fn new(layer: &'a mut Layer<'a>, p1: PointF, p2: PointF) -> Self {
        Self {
            layer,
            p1,
            p2,
            color: Color::WHITE,
            width: 1,
            opacity: Opacity::OPAQUE,
            dash_width: 0,
            dash_gap: 0,
            round_start: false,
            round_end: false,
        }
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

    pub fn dashed(mut self, dash_width: i32, gap: i32) -> Self {
        self.dash_width = dash_width;
        self.dash_gap = gap;
        self
    }

    pub fn round_start(mut self) -> Self {
        self.round_start = true;
        self
    }

    pub fn round_end(mut self) -> Self {
        self.round_end = true;
        self
    }

    pub fn rounded(mut self) -> Self {
        self.round_start = true;
        self.round_end = true;
        self
    }

    pub fn draw(mut self) {
        if self.width == 0 {
            return;
        }

        let p1_x = self.p1.x as i32;
        let p1_y = self.p1.y as i32;
        let p2_x = self.p2.x as i32;
        let p2_y = self.p2.y as i32;

        let round_start = self.round_start;
        let round_end = self.round_end;

        // Horizontal line
        if p1_y == p2_y {
            self.draw_horizontal();
        }
        // Vertical line
        else if p1_x == p2_x {
            self.draw_vertical();
        }
        // Skewed line (Bresenham's algorithm with thickness)
        else {
            self.draw_skewed();
        }

        // Draw rounded ends if requested
        if round_start || round_end {
            self.draw_round_caps(round_start, round_end);
        }
    }

    fn draw_horizontal(&mut self) {
        let w = self.width - 1;
        let w_half0 = w >> 1;
        let w_half1 = w_half0 + (w & 0x1);

        let x1 = self.p1.x.min(self.p2.x) as i32;
        let x2 = self.p1.x.max(self.p2.x) as i32;
        let y = self.p1.y as i32;

        let rect = Rect::new(x1, y - w_half1, (x2 - x1) as u32, (w + 1) as u32);

        // Simple case: no dashing
        if self.dash_gap == 0 || self.dash_width == 0 {
            let desc = BlendDescriptor {
                mode: BlendMode::Normal,
                source: Some(BlendSource::SolidColor { color: ColorAlpha::from_rgb(self.color, self.opacity) }),
                dest_rect: rect,
                opacity: self.opacity,
                mask: None,
            };
            self.layer.blend(&desc);
        } else {
            // Dashed line: draw individual segments
            let mut x = x1;
            let dash_period = self.dash_width + self.dash_gap;
            
            while x < x2 {
                let seg_end = (x + self.dash_width).min(x2);
                let seg_rect = Rect::new(x, y - w_half1, (seg_end - x) as u32, (w + 1) as u32);
                
                let desc = BlendDescriptor {
                    mode: BlendMode::Normal,
                    source: Some(BlendSource::SolidColor { color: ColorAlpha::from_rgb(self.color, self.opacity) }),
                    dest_rect: seg_rect,
                    opacity: self.opacity,
                    mask: None,
                };
                self.layer.blend(&desc);
                
                x += dash_period;
            }
        }
    }

    fn draw_vertical(&mut self) {
        let w = self.width - 1;
        let w_half0 = w >> 1;
        let w_half1 = w_half0 + (w & 0x1);

        let x = self.p1.x as i32;
        let y1 = self.p1.y.min(self.p2.y) as i32;
        let y2 = self.p1.y.max(self.p2.y) as i32;

        let rect = Rect::new(x - w_half1, y1, (w + 1) as u32, (y2 - y1) as u32);

        // Simple case: no dashing
        if self.dash_gap == 0 || self.dash_width == 0 {
            let desc = BlendDescriptor {
                mode: BlendMode::Normal,
                source: Some(BlendSource::SolidColor { color: ColorAlpha::from_rgb(self.color, self.opacity) }),
                dest_rect: rect,
                opacity: self.opacity,
                mask: None,
            };
            self.layer.blend(&desc);
        } else {
            // Dashed line: draw individual segments
            let mut y = y1;
            let dash_period = self.dash_width + self.dash_gap;
            
            while y < y2 {
                let seg_end = (y + self.dash_width).min(y2);
                let seg_rect = Rect::new(x - w_half1, y, (w + 1) as u32, (seg_end - y) as u32);
                
                let desc = BlendDescriptor {
                    mode: BlendMode::Normal,
                    source: Some(BlendSource::SolidColor { color: ColorAlpha::from_rgb(self.color, self.opacity) }),
                    dest_rect: seg_rect,
                    opacity: self.opacity,
                    mask: None,
                };
                self.layer.blend(&desc);
                
                y += dash_period;
            }
        }
    }

    fn draw_skewed(&mut self) {
        // Bresenham's line algorithm with anti-aliasing and thickness
        let dx = (self.p2.x - self.p1.x).abs();
        let dy = (self.p2.y - self.p1.y).abs();
        
        let sx = if self.p1.x < self.p2.x { 1.0 } else { -1.0 };
        let sy = if self.p1.y < self.p2.y { 1.0 } else { -1.0 };
        
        let mut err = dx - dy;
        let mut x = self.p1.x;
        let mut y = self.p1.y;
        
        let thick_half = (self.width as f32) / 2.0;
        
        // For skewed lines, we draw perpendicular rectangles at each point
        while (x - self.p2.x).abs() > 0.5 || (y - self.p2.y).abs() > 0.5 {
            // Calculate perpendicular direction (simplified without atan2)
            let dx = self.p2.x - self.p1.x;
            let dy = self.p2.y - self.p1.y;
            // Use simple normalization without sqrt
            let len = (dx * dx + dy * dy).max(1.0); // Avoid division by zero
            let scale = 1.0 / len; // Approximate without sqrt for embedded
            let perp_x = -dy * scale;
            let perp_y = dx * scale;
            
            let offset_x = (perp_x * thick_half) as i32;
            let offset_y = (perp_y * thick_half) as i32;
            
            // Draw a small rectangle at this point
            let px = x as i32;
            let py = y as i32;
            let rect = Rect::new(
                px - offset_x,
                py - offset_y,
                ((offset_x * 2).abs().max(1)) as u32,
                ((offset_y * 2).abs().max(1)) as u32
            );
            
            let desc = BlendDescriptor {
                mode: BlendMode::Normal,
                source: Some(BlendSource::SolidColor { color: ColorAlpha::from_rgb(self.color, self.opacity) }),
                dest_rect: rect,
                opacity: self.opacity,
                mask: None,
            };
            self.layer.blend(&desc);
            
            let e2 = 2.0 * err;
            if e2 > -dy {
                err -= dy;
                x += sx;
            }
            if e2 < dx {
                err += dx;
                y += sy;
            }
        }
    }

    fn draw_round_caps(&mut self, round_start: bool, round_end: bool) {
        let r = self.width / 2;
        
        if round_start {
            let circle_rect = Rect::new(
                (self.p1.x as i32) - r,
                (self.p1.y as i32) - r,
                self.width as u32,
                self.width as u32
            );
            
            // For now, draw a simple square cap (TODO: implement proper circular cap)
            let desc = BlendDescriptor {
                mode: BlendMode::Normal,
                source: Some(BlendSource::SolidColor { color: ColorAlpha::from_rgb(self.color, self.opacity) }),
                dest_rect: circle_rect,
                opacity: self.opacity,
                mask: None,
            };
            self.layer.blend(&desc);
        }
        
        if round_end {
            let circle_rect = Rect::new(
                (self.p2.x as i32) - r,
                (self.p2.y as i32) - r,
                self.width as u32,
                self.width as u32
            );
            
            let desc = BlendDescriptor {
                mode: BlendMode::Normal,
                source: Some(BlendSource::SolidColor { color: ColorAlpha::from_rgb(self.color, self.opacity) }),
                dest_rect: circle_rect,
                opacity: self.opacity,
                mask: None,
            };
            self.layer.blend(&desc);
        }
    }
}

impl<'a> Layer<'a> {
    pub fn line(&'a mut self, p1: PointF, p2: PointF) -> Line<'a> {
        Line::new(self, p1, p2)
    }

    pub fn line_from_to(&'a mut self, x1: f32, y1: f32, x2: f32, y2: f32) -> Line<'a> {
        Line::new(self, PointF::new(x1, y1), PointF::new(x2, y2))
    }
}
