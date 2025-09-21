/// Comprehensive 2D primitives with fluent API and anti-aliased rendering
/// 
/// This module provides all fundamental 2D shapes with:
/// - Fluent API for chainable method calls
/// - Anti-aliased rendering for smooth edges
/// - Gradient and solid color fills
/// - Advanced stroke options
/// - Optimized rasterization algorithms

use super::types::{Point, Size, Rect as RectGeometry, Fixed, CornerRadii, Rgba8888, AntiAliasing};
use super::paint::Paint;
use super::stroke::{Stroke, StrokeRasterizer};
use super::canvas2d::Canvas2D;
use micromath::F32Ext;

/// Drawable trait for all primitives
pub trait Drawable {
    /// Draw the primitive to a canvas
    fn draw(self, canvas: &mut Canvas2D);
}

/// Rectangle primitive with optional corner radii
#[derive(Debug, Clone)]
pub struct Rect {
    geometry: RectGeometry,
    fill: Option<Paint>,
    stroke: Option<Stroke>,
    corner_radii: CornerRadii,
    anti_aliasing: AntiAliasing,
}

impl Rect {
    /// Create new rectangle
    #[inline]
    pub fn new(top_left: Point, size: Size) -> Self {
        Self {
            geometry: RectGeometry::new(top_left, size),
            fill: None,
            stroke: None,
            corner_radii: CornerRadii::zero(),
            anti_aliasing: AntiAliasing::default(),
        }
    }
    
    /// Create rectangle from coordinates
    #[inline]
    pub fn from_coords(x: i32, y: i32, width: u32, height: u32) -> Self {
        Self::new(Point::new(x, y), Size::new(width, height))
    }
    
    /// Set fill paint
    #[inline]
    pub fn fill(mut self, paint: Paint) -> Self {
        self.fill = Some(paint);
        self
    }
    
    /// Set stroke
    #[inline]
    pub fn stroke(mut self, stroke: Stroke) -> Self {
        self.stroke = Some(stroke);
        self
    }
    
    /// Set uniform corner radius
    #[inline]
    pub fn corner_radius(mut self, radius: f32) -> Self {
        self.corner_radii = CornerRadii::from_f32(radius);
        self
    }
    
    /// Set individual corner radii
    #[inline]
    pub fn corner_radii(mut self, radii: CornerRadii) -> Self {
        self.corner_radii = radii;
        self
    }
    
    /// Set anti-aliasing quality
    #[inline]
    pub fn anti_aliasing(mut self, aa: AntiAliasing) -> Self {
        self.anti_aliasing = aa;
        self
    }
}

impl Drawable for Rect {
    fn draw(self, canvas: &mut Canvas2D) {
        // For now, use simple rectangle drawing
        if let Some(fill) = &self.fill {
            match fill {
                Paint::Solid(color) => {
                    canvas.fill_rect(
                        self.geometry.top_left.x,
                        self.geometry.top_left.y,
                        self.geometry.size.width,
                        self.geometry.size.height,
                        *color
                    );
                }
                _ => {
                    // Sample gradient for each pixel
                    for y in self.geometry.top_left.y..=self.geometry.bottom() {
                        for x in self.geometry.top_left.x..=self.geometry.right() {
                            let color = fill.sample_at(Point::new(x, y));
                            canvas.set_pixel(x, y, color);
                        }
                    }
                }
            }
        }
        
        // TODO: Implement stroke drawing
    }
}

/// Circle primitive
#[derive(Debug, Clone)]
pub struct Circle {
    center: Point,
    radius: Fixed,
    fill: Option<Paint>,
    stroke: Option<Stroke>,
    anti_aliasing: AntiAliasing,
}

impl Circle {
    /// Create new circle
    #[inline]
    pub fn new(center: Point, radius: f32) -> Self {
        Self {
            center,
            radius: Fixed::from_f32(radius.max(0.0)),
            fill: None,
            stroke: None,
            anti_aliasing: AntiAliasing::default(),
        }
    }
    
    /// Set fill paint
    #[inline]
    pub fn fill(mut self, paint: Paint) -> Self {
        self.fill = Some(paint);
        self
    }
    
    /// Set stroke
    #[inline]
    pub fn stroke(mut self, stroke: Stroke) -> Self {
        self.stroke = Some(stroke);
        self
    }
    
    /// Set anti-aliasing quality
    #[inline]
    pub fn anti_aliasing(mut self, aa: AntiAliasing) -> Self {
        self.anti_aliasing = aa;
        self
    }
}

impl Drawable for Circle {
    fn draw(self, canvas: &mut Canvas2D) {
        let radius_int = self.radius.to_int();
        
        if let Some(fill) = &self.fill {
            // Anti-aliased filled circle
            for y in (self.center.y - radius_int - 1)..=(self.center.y + radius_int + 1) {
                for x in (self.center.x - radius_int - 1)..=(self.center.x + radius_int + 1) {
                    if x >= 0 && y >= 0 && x < canvas.width() as i32 && y < canvas.height() as i32 {
                        let dx = x - self.center.x;
                        let dy = y - self.center.y;
                        let dist = ((dx * dx + dy * dy) as f32).sqrt();
                        let radius_f = radius_int as f32;
                        
                        if dist <= radius_f + 1.0 {
                            let mut color = fill.sample_at(Point::new(x, y));
                            
                            // Anti-aliasing on the edge
                            if dist > radius_f - 1.0 {
                                let edge_factor = (radius_f + 1.0 - dist).max(0.0).min(1.0);
                                color.a = (color.a as f32 * edge_factor) as u8;
                            }
                            
                            if color.a > 0 {
                                canvas.set_pixel(x, y, color);
                            }
                        }
                    }
                }
            }
        }
        
        // Anti-aliased stroke
        if let Some(stroke) = &self.stroke {
            let stroke_width = stroke.effective_width().to_int().max(1);
            let half_stroke = stroke_width / 2;
            let inner_radius = radius_int - half_stroke;
            let outer_radius = radius_int + half_stroke;
            
            for y in (self.center.y - outer_radius - 1)..=(self.center.y + outer_radius + 1) {
                for x in (self.center.x - outer_radius - 1)..=(self.center.x + outer_radius + 1) {
                    if x >= 0 && y >= 0 && x < canvas.width() as i32 && y < canvas.height() as i32 {
                        let dx = x - self.center.x;
                        let dy = y - self.center.y;
                        let dist = ((dx * dx + dy * dy) as f32).sqrt();
                        
                        if dist >= inner_radius as f32 - 1.0 && dist <= outer_radius as f32 + 1.0 {
                            let mut color = match &stroke.paint {
                                Paint::Solid(c) => *c,
                                _ => stroke.paint.sample_at(Point::new(x, y)),
                            };
                            
                            // Anti-aliasing on both edges
                            let inner_edge = (dist - (inner_radius as f32 - 1.0)).max(0.0).min(1.0);
                            let outer_edge = ((outer_radius as f32 + 1.0) - dist).max(0.0).min(1.0);
                            let edge_factor = inner_edge.min(outer_edge);
                            
                            color.a = (color.a as f32 * edge_factor) as u8;
                            
                            if color.a > 0 {
                                canvas.set_pixel(x, y, color);
                            }
                        }
                    }
                }
            }
        }
    }
}

/// Line primitive
#[derive(Debug, Clone)]
pub struct Line {
    start: Point,
    end: Point,
    stroke: Option<Stroke>,
}

impl Line {
    /// Create new line
    #[inline]
    pub fn new(start: Point, end: Point) -> Self {
        Self {
            start,
            end,
            stroke: None,
        }
    }
    
    /// Set stroke
    #[inline]
    pub fn stroke(mut self, stroke: Stroke) -> Self {
        self.stroke = Some(stroke);
        self
    }
}

impl Drawable for Line {
    fn draw(self, canvas: &mut Canvas2D) {
        if let Some(stroke) = &self.stroke {
            let color = match &stroke.paint {
                Paint::Solid(c) => *c,
                _ => stroke.paint.sample_at(self.start),
            };
            
            let stroke_width = stroke.effective_width().to_int().max(1);
            let half_width = stroke_width / 2;
            
            // Bresenham's line algorithm with anti-aliased thickness
            let dx = (self.end.x - self.start.x).abs();
            let dy = (self.end.y - self.start.y).abs();
            let sx = if self.start.x < self.end.x { 1 } else { -1 };
            let sy = if self.start.y < self.end.y { 1 } else { -1 };
            let mut err = dx - dy;
            
            let mut x = self.start.x;
            let mut y = self.start.y;
            
            loop {
                // Draw thick anti-aliased point
                for dy_offset in -half_width..=half_width {
                    for dx_offset in -half_width..=half_width {
                        let px = x + dx_offset;
                        let py = y + dy_offset;
                        
                        if px >= 0 && py >= 0 && px < canvas.width() as i32 && py < canvas.height() as i32 {
                            // Simple distance-based anti-aliasing
                            let dist_sq = dx_offset * dx_offset + dy_offset * dy_offset;
                            let radius_sq = half_width * half_width;
                            
                            if dist_sq <= radius_sq + half_width {
                                let mut aa_color = color;
                                
                                // Anti-aliasing on the edge
                                if dist_sq > radius_sq {
                                    let edge_factor = 1.0 - ((dist_sq - radius_sq) as f32 / half_width as f32).min(1.0);
                                    aa_color.a = (color.a as f32 * edge_factor) as u8;
                                }
                                
                                if aa_color.a > 0 {
                                    canvas.set_pixel(px, py, aa_color);
                                }
                            }
                        }
                    }
                }
                
                if x == self.end.x && y == self.end.y { break; }
                
                let e2 = 2 * err;
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
    }
}

/// Arc primitive
#[derive(Debug, Clone)]
pub struct Arc {
    center: Point,
    radius: Fixed,
    start_angle: Fixed,
    end_angle: Fixed,
    stroke: Option<Stroke>,
    anti_aliasing: AntiAliasing,
}

impl Arc {
    /// Create new arc
    #[inline]
    pub fn new(center: Point, radius: f32, start_angle: f32, end_angle: f32) -> Self {
        Self {
            center,
            radius: Fixed::from_f32(radius.max(0.0)),
            start_angle: Fixed::from_f32(start_angle),
            end_angle: Fixed::from_f32(end_angle),
            stroke: None,
            anti_aliasing: AntiAliasing::default(),
        }
    }
    
    /// Set stroke
    #[inline]
    pub fn stroke(mut self, stroke: Stroke) -> Self {
        self.stroke = Some(stroke);
        self
    }
    
    /// Set anti-aliasing quality
    #[inline]
    pub fn anti_aliasing(mut self, aa: AntiAliasing) -> Self {
        self.anti_aliasing = aa;
        self
    }
}

impl Drawable for Arc {
    fn draw(self, canvas: &mut Canvas2D) {
        if let Some(stroke) = &self.stroke {
            // Simple arc drawing with safe iteration
            let color = match &stroke.paint {
                Paint::Solid(c) => *c,
                _ => stroke.paint.sample_at(self.center),
            };
            
            let radius_int = self.radius.to_int();
            let start_rad = self.start_angle.to_f32();
            let end_rad = self.end_angle.to_f32();
            
            // Calculate arc length and step size for smooth rendering
            let mut arc_length = end_rad - start_rad;
            if arc_length < 0.0 {
                arc_length += 2.0 * core::f32::consts::PI;
            }
            
            // Limit arc length to prevent infinite loops
            arc_length = arc_length.min(2.0 * core::f32::consts::PI);
            
            let steps = (arc_length * radius_int as f32 * 0.5).max(1.0) as i32;
            let step_size = arc_length / steps as f32;
            
            // Draw arc with anti-aliasing consideration
            for i in 0..=steps {
                let angle = start_rad + i as f32 * step_size;
                let x = self.center.x + (radius_int as f32 * angle.cos()) as i32;
                let y = self.center.y + (radius_int as f32 * angle.sin()) as i32;
                
                // Simple anti-aliasing: draw slightly thicker lines
                let stroke_width = stroke.effective_width().to_int().max(1);
                let half_width = stroke_width / 2;
                
                for dy in -half_width..=half_width {
                    for dx in -half_width..=half_width {
                        let px = x + dx;
                        let py = y + dy;
                        
                        if px >= 0 && py >= 0 && px < canvas.width() as i32 && py < canvas.height() as i32 {
                            // Simple distance-based anti-aliasing
                            let dist_sq = dx * dx + dy * dy;
                            let radius_sq = half_width * half_width;
                            
                            if dist_sq <= radius_sq {
                                let alpha = if dist_sq == 0 {
                                    color.a
                                } else {
                                    let dist_factor = 1.0 - (dist_sq as f32 / radius_sq as f32).sqrt();
                                    (color.a as f32 * dist_factor) as u8
                                };
                                
                                let aa_color = Rgba8888::new(color.r, color.g, color.b, alpha);
                                canvas.set_pixel(px, py, aa_color);
                            }
                        }
                    }
                }
            }
        }
    }
}

/// Bezier curve primitive (quadratic and cubic)
#[derive(Debug, Clone)]
pub struct Bezier {
    points: heapless::Vec<Point, 4>, // Support up to cubic bezier
    stroke: Option<Stroke>,
    anti_aliasing: AntiAliasing,
}

impl Bezier {
    /// Create quadratic bezier curve
    #[inline]
    pub fn quadratic(start: Point, control: Point, end: Point) -> Self {
        let mut points = heapless::Vec::new();
        let _ = points.push(start);
        let _ = points.push(control);
        let _ = points.push(end);
        
        Self {
            points,
            stroke: None,
            anti_aliasing: AntiAliasing::default(),
        }
    }
    
    /// Create cubic bezier curve
    #[inline]
    pub fn cubic(start: Point, control1: Point, control2: Point, end: Point) -> Self {
        let mut points = heapless::Vec::new();
        let _ = points.push(start);
        let _ = points.push(control1);
        let _ = points.push(control2);
        let _ = points.push(end);
        
        Self {
            points,
            stroke: None,
            anti_aliasing: AntiAliasing::default(),
        }
    }
    
    /// Set stroke
    #[inline]
    pub fn stroke(mut self, stroke: Stroke) -> Self {
        self.stroke = Some(stroke);
        self
    }
    
    /// Set anti-aliasing quality
    #[inline]
    pub fn anti_aliasing(mut self, aa: AntiAliasing) -> Self {
        self.anti_aliasing = aa;
        self
    }
}

impl Drawable for Bezier {
    fn draw(self, canvas: &mut Canvas2D) {
        if let Some(stroke) = &self.stroke {
            // Simple bezier approximation using line segments
            let color = match &stroke.paint {
                Paint::Solid(c) => *c,
                _ => stroke.paint.sample_at(self.points[0]),
            };
            
            if self.points.len() < 3 {
                return;
            }
            
            let steps = 50; // Number of line segments to approximate curve
            let mut prev_point = self.points[0];
            
            for i in 1..=steps {
                let t = i as f32 / steps as f32;
                let current_point = if self.points.len() == 3 {
                    // Quadratic bezier
                    let p0 = self.points[0];
                    let p1 = self.points[1];
                    let p2 = self.points[2];
                    let t2 = t * t;
                    let mt = 1.0 - t;
                    let mt2 = mt * mt;
                    
                    Point::new(
                        (mt2 * p0.x as f32 + 2.0 * mt * t * p1.x as f32 + t2 * p2.x as f32) as i32,
                        (mt2 * p0.y as f32 + 2.0 * mt * t * p1.y as f32 + t2 * p2.y as f32) as i32,
                    )
                } else {
                    // Cubic bezier
                    let p0 = self.points[0];
                    let p1 = self.points[1];
                    let p2 = self.points[2];
                    let p3 = self.points[3];
                    let t2 = t * t;
                    let t3 = t2 * t;
                    let mt = 1.0 - t;
                    let mt2 = mt * mt;
                    let mt3 = mt2 * mt;
                    
                    Point::new(
                        (mt3 * p0.x as f32 + 3.0 * mt2 * t * p1.x as f32 + 3.0 * mt * t2 * p2.x as f32 + t3 * p3.x as f32) as i32,
                        (mt3 * p0.y as f32 + 3.0 * mt2 * t * p1.y as f32 + 3.0 * mt * t2 * p2.y as f32 + t3 * p3.y as f32) as i32,
                    )
                };
                
                // Draw line from prev_point to current_point
                Line::new(prev_point, current_point)
                    .stroke(stroke.clone())
                    .draw(canvas);
                    
                prev_point = current_point;
            }
        }
    }
}

/// Rectangle rasterizer with corner radius support
pub struct RectRasterizer {
    // Temporary storage for optimization
}

impl RectRasterizer {
    pub fn new() -> Self {
        Self {}
    }
    
    /// Rasterize rectangle fill with corner radii
    pub fn rasterize_fill<R: crate::libs::gfx::two_d::Rasterizer >(
        &mut self,
        raster: &mut R,
        rect: &RectGeometry,
        corner_radii: &CornerRadii,
        paint: &Paint,
        aa: AntiAliasing,
    ) {
        if !corner_radii.has_radius() {
            // Simple rectangle fill
            self.rasterize_simple_fill(raster, rect, paint);
            return;
        }
        
        // Complex rectangle with corner radii
        self.rasterize_rounded_fill(raster, rect, corner_radii, paint, aa);
    }
    
    /// Rasterize simple rectangle fill
    fn rasterize_simple_fill<R: crate::libs::gfx::two_d::Rasterizer>(
        &self,
        raster: &mut R,
        rect: &RectGeometry,
        paint: &Paint,
    ) {
        match paint {
            Paint::Solid(color) => {
                raster.set_pixels_rect(*rect, *color);
            }
            _ => {
                // Sample gradient for each pixel
                for y in rect.top_left.y..=rect.bottom() {
                    for x in rect.top_left.x..=rect.right() {
                        let color = paint.sample_at(Point::new(x, y));
                        raster.set_pixel(x, y, color);
                    }
                }
            }
        }
    }
    
    /// Rasterize rounded rectangle fill with anti-aliasing
    fn rasterize_rounded_fill<R: crate::libs::gfx::two_d::Rasterizer>(
        &mut self,
        raster: &mut R,
        rect: &RectGeometry,
        corner_radii: &CornerRadii,
        paint: &Paint,
        aa: AntiAliasing,
    ) {
        let samples = match aa {
            AntiAliasing::None => 1,
            AntiAliasing::Low => 4,
            AntiAliasing::Medium => 16,
            AntiAliasing::High => 64,
        };
        
        let samples_per_axis = (samples as f32).sqrt() as i32;
        let step = Fixed::ONE / Fixed::from_int(samples_per_axis);
        
        for y in rect.top_left.y..=rect.bottom() {
            for x in rect.top_left.x..=rect.right() {
                let mut covered_samples = 0;
                
                // Multi-sample for anti-aliasing
                for sy in 0..samples_per_axis {
                    for sx in 0..samples_per_axis {
                        let sample_x = Fixed::from_int(x) + step * Fixed::from_int(sx) + step / Fixed::from_int(2);
                        let sample_y = Fixed::from_int(y) + step * Fixed::from_int(sy) + step / Fixed::from_int(2);
                        
                        if self.point_in_rounded_rect(Point::from_fixed(sample_x, sample_y), rect, corner_radii) {
                            covered_samples += 1;
                        }
                    }
                }
                
                if covered_samples > 0 {
                    let coverage = ((covered_samples * 255) / samples) as u8;
                    let color = paint.sample_at(Point::new(x, y));
                    
                    if coverage == 255 {
                        raster.set_pixel(x, y, color);
                    } else {
                        raster.blend_pixel(x, y, color, coverage);
                    }
                }
            }
        }
    }
    
    /// Check if point is inside rounded rectangle
    fn point_in_rounded_rect(&self, point: Point, rect: &RectGeometry, corner_radii: &CornerRadii) -> bool {
        let px = point.x;
        let py = point.y;
        
        // Check if point is in main rectangle area
        if px >= rect.top_left.x && px <= rect.right() && py >= rect.top_left.y && py <= rect.bottom() {
            // Check corner regions
            let left = rect.top_left.x;
            let right = rect.right();
            let top = rect.top_left.y;
            let bottom = rect.bottom();
            
            // Top-left corner
            if px < left + corner_radii.top_left.to_int() && py < top + corner_radii.top_left.to_int() {
                let cx = left + corner_radii.top_left.to_int();
                let cy = top + corner_radii.top_left.to_int();
                let dist_sq = (px - cx) * (px - cx) + (py - cy) * (py - cy);
                let radius_sq = corner_radii.top_left.to_int() * corner_radii.top_left.to_int();
                return dist_sq <= radius_sq;
            }
            
            // Top-right corner
            if px > right - corner_radii.top_right.to_int() && py < top + corner_radii.top_right.to_int() {
                let cx = right - corner_radii.top_right.to_int();
                let cy = top + corner_radii.top_right.to_int();
                let dist_sq = (px - cx) * (px - cx) + (py - cy) * (py - cy);
                let radius_sq = corner_radii.top_right.to_int() * corner_radii.top_right.to_int();
                return dist_sq <= radius_sq;
            }
            
            // Bottom-right corner
            if px > right - corner_radii.bottom_right.to_int() && py > bottom - corner_radii.bottom_right.to_int() {
                let cx = right - corner_radii.bottom_right.to_int();
                let cy = bottom - corner_radii.bottom_right.to_int();
                let dist_sq = (px - cx) * (px - cx) + (py - cy) * (py - cy);
                let radius_sq = corner_radii.bottom_right.to_int() * corner_radii.bottom_right.to_int();
                return dist_sq <= radius_sq;
            }
            
            // Bottom-left corner
            if px < left + corner_radii.bottom_left.to_int() && py > bottom - corner_radii.bottom_left.to_int() {
                let cx = left + corner_radii.bottom_left.to_int();
                let cy = bottom - corner_radii.bottom_left.to_int();
                let dist_sq = (px - cx) * (px - cx) + (py - cy) * (py - cy);
                let radius_sq = corner_radii.bottom_left.to_int() * corner_radii.bottom_left.to_int();
                return dist_sq <= radius_sq;
            }
            
            // Point is in main rectangle area, not in corner regions
            return true;
        }
        
        false
    }
    
    /// Rasterize rectangle stroke
    pub fn rasterize_stroke<R: crate::libs::gfx::two_d::Rasterizer >(
        &mut self,
        raster: &mut R,
        rect: &RectGeometry,
        corner_radii: &CornerRadii,
        stroke: &Stroke,
    ) {
        let mut stroke_rasterizer = StrokeRasterizer::new();
        
        if !corner_radii.has_radius() {
            // Simple rectangle stroke - draw four lines
            let top_left = rect.top_left;
            let top_right = Point::new(rect.right(), rect.top_left.y);
            let bottom_right = Point::new(rect.right(), rect.bottom());
            let bottom_left = Point::new(rect.top_left.x, rect.bottom());
            
            stroke_rasterizer.rasterize_line(raster, top_left, top_right, stroke);
            stroke_rasterizer.rasterize_line(raster, top_right, bottom_right, stroke);
            stroke_rasterizer.rasterize_line(raster, bottom_right, bottom_left, stroke);
            stroke_rasterizer.rasterize_line(raster, bottom_left, top_left, stroke);
        } else {
            // Rounded rectangle stroke - more complex
            self.rasterize_rounded_stroke(raster, rect, corner_radii, stroke);
        }
    }
    
    /// Rasterize rounded rectangle stroke
    fn rasterize_rounded_stroke<R: crate::libs::gfx::two_d::Rasterizer>(
        &mut self,
        raster: &mut R,
        rect: &RectGeometry,
        corner_radii: &CornerRadii,
        stroke: &Stroke,
    ) {
        // For now, use a simplified approach - draw the outline pixel by pixel
        // A more sophisticated implementation would use proper arc rendering
        
        let half_width = stroke.effective_width() / Fixed::from_int(2);
        let width_int = half_width.to_int();
        
        for y in rect.top_left.y - width_int..=rect.bottom() + width_int {
            for x in rect.top_left.x - width_int..=rect.right() + width_int {
                let point = Point::new(x, y);
                
                // Check if point is on the stroke boundary
                let inside_outer = self.point_in_rounded_rect(point, rect, corner_radii);
                
                // Create inner rectangle
                let inner_rect = RectGeometry::new(
                    Point::new(rect.top_left.x + width_int, rect.top_left.y + width_int),
                    Size::new(
                        rect.size.width.saturating_sub(2 * width_int as u32),
                        rect.size.height.saturating_sub(2 * width_int as u32),
                    ),
                );
                
                let inside_inner = if inner_rect.size.width > 0 && inner_rect.size.height > 0 {
                    self.point_in_rounded_rect(point, &inner_rect, corner_radii)
                } else {
                    false
                };
                
                if inside_outer && !inside_inner {
                    let color = stroke.paint.sample_at(point);
                    raster.set_pixel(x, y, color);
                }
            }
        }
    }
}

/// Circle rasterizer with anti-aliasing
pub struct CircleRasterizer;

impl CircleRasterizer {
    pub fn new() -> Self {
        Self
    }
    
    /// Rasterize circle fill
    pub fn rasterize_fill<R: crate::libs::gfx::two_d::Rasterizer >(
        &mut self,
        raster: &mut R,
        center: Point,
        radius: Fixed,
        paint: &Paint,
        aa: AntiAliasing,
    ) {
        let radius_int = radius.to_int();
        let radius_sq = radius_int * radius_int;
        
        let min_x = (center.x - radius_int - 1).max(0);
        let max_x = (center.x + radius_int + 1).min(raster.width() as i32 - 1);
        let min_y = (center.y - radius_int - 1).max(0);
        let max_y = (center.y + radius_int + 1).min(raster.height() as i32 - 1);
        
        let samples = match aa {
            AntiAliasing::None => 1,
            AntiAliasing::Low => 4,
            AntiAliasing::Medium => 16,
            AntiAliasing::High => 64,
        };
        
        let samples_per_axis = (samples as f32).sqrt() as i32;
        let step = Fixed::ONE / Fixed::from_int(samples_per_axis);
        
        for y in min_y..=max_y {
            for x in min_x..=max_x {
                let mut covered_samples = 0;
                
                for sy in 0..samples_per_axis {
                    for sx in 0..samples_per_axis {
                        let sample_x = Fixed::from_int(x) + step * Fixed::from_int(sx) + step / Fixed::from_int(2);
                        let sample_y = Fixed::from_int(y) + step * Fixed::from_int(sy) + step / Fixed::from_int(2);
                        
                        let dx = sample_x.to_int() - center.x;
                        let dy = sample_y.to_int() - center.y;
                        let dist_sq = dx * dx + dy * dy;
                        
                        if dist_sq <= radius_sq {
                            covered_samples += 1;
                        }
                    }
                }
                
                if covered_samples > 0 {
                    let coverage = ((covered_samples * 255) / samples) as u8;
                    let color = paint.sample_at(Point::new(x, y));
                    
                    if coverage == 255 {
                        raster.set_pixel(x, y, color);
                    } else {
                        raster.blend_pixel(x, y, color, coverage);
                    }
                }
            }
        }
    }
    
    /// Rasterize circle stroke
    pub fn rasterize_stroke<R: crate::libs::gfx::two_d::Rasterizer >(
        &mut self,
        raster: &mut R,
        center: Point,
        radius: Fixed,
        stroke: &Stroke,
    ) {
        // Use Bresenham's circle algorithm with stroke width
        let radius_int = radius.to_int();
        let stroke_width = stroke.effective_width().to_int();
        let half_stroke = stroke_width / 2;
        
        // Draw circle outline with thickness
        for angle_deg in 0..360 {
            let angle_rad = (angle_deg as f32) * core::f32::consts::PI / 180.0;
            let cos_a = angle_rad.cos();
            let sin_a = angle_rad.sin();
            
            let x = center.x + (radius_int as f32 * cos_a) as i32;
            let y = center.y + (radius_int as f32 * sin_a) as i32;
            
            // Draw thick point
            for dy in -half_stroke..=half_stroke {
                for dx in -half_stroke..=half_stroke {
                    let px = x + dx;
                    let py = y + dy;
                    
                    if px >= 0 && py >= 0 && px < raster.width() as i32 && py < raster.height() as i32 {
                        let color = stroke.paint.sample_at(Point::new(px, py));
                        raster.set_pixel(px, py, color);
                    }
                }
            }
        }
    }
}

/// Arc rasterizer
pub struct ArcRasterizer;

impl ArcRasterizer {
    pub fn new() -> Self {
        Self
    }
    
    /// Rasterize arc stroke
    pub fn rasterize_arc<R: crate::libs::gfx::two_d::Rasterizer >(
        &mut self,
        raster: &mut R,
        center: Point,
        radius: Fixed,
        start_angle: Fixed,
        end_angle: Fixed,
        stroke: &Stroke,
    ) {
        let radius_int = radius.to_int();
        let stroke_width = stroke.effective_width().to_int();
        let half_stroke = stroke_width / 2;
        
        let start_deg = (start_angle.to_f32() * 180.0 / core::f32::consts::PI) as i32;
        let end_deg = (end_angle.to_f32() * 180.0 / core::f32::consts::PI) as i32;
        
        let mut angle = start_deg;
        while angle != end_deg {
            let angle_rad = (angle as f32) * core::f32::consts::PI / 180.0;
            let cos_a = angle_rad.cos();
            let sin_a = angle_rad.sin();
            
            let x = center.x + (radius_int as f32 * cos_a) as i32;
            let y = center.y + (radius_int as f32 * sin_a) as i32;
            
            // Draw thick point
            for dy in -half_stroke..=half_stroke {
                for dx in -half_stroke..=half_stroke {
                    let px = x + dx;
                    let py = y + dy;
                    
                    if px >= 0 && py >= 0 && px < raster.width() as i32 && py < raster.height() as i32 {
                        let color = stroke.paint.sample_at(Point::new(px, py));
                        raster.set_pixel(px, py, color);
                    }
                }
            }
            
            angle = (angle + 1) % 360;
            if angle == start_deg { break; } // Prevent infinite loop
        }
    }
}

/// Bezier curve rasterizer
pub struct BezierRasterizer;

impl BezierRasterizer {
    pub fn new() -> Self {
        Self
    }
    
    /// Rasterize bezier curve stroke
    pub fn rasterize_bezier<R: crate::libs::gfx::two_d::Rasterizer >(
        &mut self,
        raster: &mut R,
        points: &heapless::Vec<Point, 4>,
        stroke: &Stroke,
    ) {
        if points.len() < 3 {
            return;
        }
        
        let mut stroke_rasterizer = StrokeRasterizer::new();
        let steps = 100; // Number of line segments to approximate curve
        
        if points.len() == 3 {
            // Quadratic bezier
            let p0 = points[0];
            let p1 = points[1];
            let p2 = points[2];
            
            let mut prev_point = p0;
            
            for i in 1..=steps {
                let t = i as f32 / steps as f32;
                let t2 = t * t;
                let mt = 1.0 - t;
                let mt2 = mt * mt;
                
                let x = (mt2 * p0.x as f32 + 2.0 * mt * t * p1.x as f32 + t2 * p2.x as f32) as i32;
                let y = (mt2 * p0.y as f32 + 2.0 * mt * t * p1.y as f32 + t2 * p2.y as f32) as i32;
                
                let current_point = Point::new(x, y);
                stroke_rasterizer.rasterize_line(raster, prev_point, current_point, stroke);
                prev_point = current_point;
            }
        } else if points.len() == 4 {
            // Cubic bezier
            let p0 = points[0];
            let p1 = points[1];
            let p2 = points[2];
            let p3 = points[3];
            
            let mut prev_point = p0;
            
            for i in 1..=steps {
                let t = i as f32 / steps as f32;
                let t2 = t * t;
                let t3 = t2 * t;
                let mt = 1.0 - t;
                let mt2 = mt * mt;
                let mt3 = mt2 * mt;
                
                let x = (mt3 * p0.x as f32 + 3.0 * mt2 * t * p1.x as f32 + 3.0 * mt * t2 * p2.x as f32 + t3 * p3.x as f32) as i32;
                let y = (mt3 * p0.y as f32 + 3.0 * mt2 * t * p1.y as f32 + 3.0 * mt * t2 * p2.y as f32 + t3 * p3.y as f32) as i32;
                
                let current_point = Point::new(x, y);
                stroke_rasterizer.rasterize_line(raster, prev_point, current_point, stroke);
                prev_point = current_point;
            }
        }
    }
}
