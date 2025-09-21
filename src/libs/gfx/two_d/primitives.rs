/// Comprehensive 2D primitives with fluent API and anti-aliased rendering
/// 
/// This module provides all fundamental 2D shapes with:
/// - Fluent API for chainable method calls
/// - Anti-aliased rendering for smooth edges
/// - Gradient and solid color fills
/// - Advanced stroke options
/// - Optimized rasterization algorithms

use super::types::{Point, Size, Rect as RectGeometry, CornerRadii, Rgba8888, AntiAliasing};
use fixed::{FixedI32, types::extra::U16};
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
    
    /// Set anti-aliasing quality (alias for anti_aliasing)
    #[inline]
    pub fn aa(mut self, aa: AntiAliasing) -> Self {
        self.anti_aliasing(aa)
    }
}

impl Drawable for Rect {
    fn draw(self, canvas: &mut Canvas2D) {
        if let Some(fill) = &self.fill {
            if self.corner_radii.has_radius() {
                // Draw rounded rectangle
                self.draw_rounded_fill(canvas, fill);
            } else {
                // Draw simple rectangle
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
        }
        
        if let Some(stroke) = &self.stroke {
            if self.corner_radii.has_radius() {
                // Draw rounded stroke
                self.draw_rounded_stroke(canvas, stroke);
            } else {
                // Draw simple rectangle stroke
                self.draw_simple_stroke(canvas, stroke);
            }
        }
    }
}

impl Rect {
    /// Draw rounded rectangle fill with fast anti-aliasing
    fn draw_rounded_fill(&self, canvas: &mut Canvas2D, fill: &Paint) {
        let bounds = self.geometry;
        
        for y in bounds.top_left.y..=bounds.bottom() {
            for x in bounds.top_left.x..=bounds.right() {
                let coverage = self.calculate_rounded_coverage(x, y);
                if coverage > 0.0 {
                    let mut color = fill.sample_at(Point::new(x, y));
                    color.a = (color.a as f32 * coverage) as u8;
                    if color.a > 0 {
                        canvas.set_pixel(x, y, color);
                    }
                }
            }
        }
    }
    
    /// Draw rounded rectangle stroke
    fn draw_rounded_stroke(&self, canvas: &mut Canvas2D, stroke: &Stroke) {
        let stroke_width = stroke.effective_width().to_num::<f32>();
        let half_stroke = stroke_width / 2.0;
        
        // Create outer and inner rectangles
        let outer_bounds = RectGeometry::new(
            Point::new(
                self.geometry.top_left.x - half_stroke as i32,
                self.geometry.top_left.y - half_stroke as i32
            ),
            Size::new(
                self.geometry.size.width + (2.0 * half_stroke) as u32,
                self.geometry.size.height + (2.0 * half_stroke) as u32
            )
        );
        
        let inner_bounds = RectGeometry::new(
            Point::new(
                self.geometry.top_left.x + half_stroke as i32,
                self.geometry.top_left.y + half_stroke as i32
            ),
            Size::new(
                self.geometry.size.width.saturating_sub(stroke_width as u32),
                self.geometry.size.height.saturating_sub(stroke_width as u32)
            )
        );
        
        // Expand corner radii for outer rectangle
        let outer_radii = CornerRadii::new(
            self.corner_radii.top_left.to_num::<f32>() + half_stroke,
            self.corner_radii.top_right.to_num::<f32>() + half_stroke,
            self.corner_radii.bottom_right.to_num::<f32>() + half_stroke,
            self.corner_radii.bottom_left.to_num::<f32>() + half_stroke,
        );
        
        // Shrink corner radii for inner rectangle
        let inner_radii = CornerRadii::new(
            (self.corner_radii.top_left.to_num::<f32>() - half_stroke).max(0.0),
            (self.corner_radii.top_right.to_num::<f32>() - half_stroke).max(0.0),
            (self.corner_radii.bottom_right.to_num::<f32>() - half_stroke).max(0.0),
            (self.corner_radii.bottom_left.to_num::<f32>() - half_stroke).max(0.0),
        );
        
        for y in outer_bounds.top_left.y..=outer_bounds.bottom() {
            for x in outer_bounds.top_left.x..=outer_bounds.right() {
                let outer_coverage = self.calculate_rounded_coverage_for_rect(x, y, &outer_bounds, &outer_radii);
                let inner_coverage = if inner_bounds.size.width > 0 && inner_bounds.size.height > 0 {
                    self.calculate_rounded_coverage_for_rect(x, y, &inner_bounds, &inner_radii)
                } else {
                    0.0
                };
                
                let coverage = outer_coverage - inner_coverage;
                if coverage > 0.0 {
                    let mut color = match &stroke.paint {
                        Paint::Solid(c) => *c,
                        _ => stroke.paint.sample_at(Point::new(x, y)),
                    };
                    color.a = (color.a as f32 * coverage) as u8;
                    if color.a > 0 {
                        canvas.set_pixel(x, y, color);
                    }
                }
            }
        }
    }
    
    /// Draw simple rectangle stroke
    fn draw_simple_stroke(&self, canvas: &mut Canvas2D, stroke: &Stroke) {
        let stroke_width = stroke.effective_width().to_num::<i32>().max(1);
        let bounds = &self.geometry;
        
        let color = match &stroke.paint {
            Paint::Solid(c) => *c,
            _ => stroke.paint.sample_at(bounds.top_left),
        };
        
        // Draw top and bottom edges
        for i in 0..stroke_width {
            // Top edge
            canvas.fill_rect(
                bounds.top_left.x,
                bounds.top_left.y + i,
                bounds.size.width,
                1,
                color
            );
            // Bottom edge
            canvas.fill_rect(
                bounds.top_left.x,
                bounds.bottom() - i,
                bounds.size.width,
                1,
                color
            );
        }
        
        // Draw left and right edges
        for i in 0..stroke_width {
            // Left edge
            canvas.fill_rect(
                bounds.top_left.x + i,
                bounds.top_left.y,
                1,
                bounds.size.height,
                color
            );
            // Right edge
            canvas.fill_rect(
                bounds.right() - i,
                bounds.top_left.y,
                1,
                bounds.size.height,
                color
            );
        }
    }
    
    /// Calculate coverage for rounded rectangle using fast analytical method
    fn calculate_rounded_coverage(&self, x: i32, y: i32) -> f32 {
        self.calculate_rounded_coverage_for_rect(x, y, &self.geometry, &self.corner_radii)
    }
    
    /// Calculate coverage for any rounded rectangle
    fn calculate_rounded_coverage_for_rect(&self, x: i32, y: i32, rect: &RectGeometry, radii: &CornerRadii) -> f32 {
        let px = x as f32 + 0.5;
        let py = y as f32 + 0.5;
        
        let left = rect.top_left.x as f32;
        let top = rect.top_left.y as f32;
        let right = rect.right() as f32;
        let bottom = rect.bottom() as f32;
        
        // CSS-like clamping: radius can't exceed half the rectangle's dimensions
        let width = right - left;
        let height = bottom - top;
        let max_radius = if width < height { width / 2.0 } else { height / 2.0 };
        
        // Clamp radii like CSS does
        let clamped_radii = CornerRadii {
            top_left: FixedI32::<U16>::from_num(if radii.top_left.to_num::<f32>() > max_radius { max_radius } else { radii.top_left.to_num::<f32>() }),
            top_right: FixedI32::<U16>::from_num(if radii.top_right.to_num::<f32>() > max_radius { max_radius } else { radii.top_right.to_num::<f32>() }),
            bottom_right: FixedI32::<U16>::from_num(if radii.bottom_right.to_num::<f32>() > max_radius { max_radius } else { radii.bottom_right.to_num::<f32>() }),
            bottom_left: FixedI32::<U16>::from_num(if radii.bottom_left.to_num::<f32>() > max_radius { max_radius } else { radii.bottom_left.to_num::<f32>() }),
        };
        let radii = &clamped_radii;
        
        // Check if point is outside the rectangle bounds
        if px < left || px > right || py < top || py > bottom {
            return 0.0;
        }
        
        // Determine which corner region we're in and get the appropriate radius
        let (corner_radius, corner_x, corner_y) = if px <= left + radii.top_left.to_num::<f32>() && py <= top + radii.top_left.to_num::<f32>() {
            // Top-left corner region
            let radius = radii.top_left.to_num::<f32>();
            (radius, left + radius, top + radius)
        } else if px >= right - radii.top_right.to_num::<f32>() && py <= top + radii.top_right.to_num::<f32>() {
            // Top-right corner region
            let radius = radii.top_right.to_num::<f32>();
            (radius, right - radius, top + radius)
        } else if px >= right - radii.bottom_right.to_num::<f32>() && py >= bottom - radii.bottom_right.to_num::<f32>() {
            // Bottom-right corner region
            let radius = radii.bottom_right.to_num::<f32>();
            (radius, right - radius, bottom - radius)
        } else if px <= left + radii.bottom_left.to_num::<f32>() && py >= bottom - radii.bottom_left.to_num::<f32>() {
            // Bottom-left corner region
            let radius = radii.bottom_left.to_num::<f32>();
            (radius, left + radius, bottom - radius)
        } else {
            // Not in any corner region - always fully covered
            return 1.0;
        };
        
        // If no radius, it's a sharp corner
        if corner_radius <= 0.0 {
            return 1.0;
        }
        
        // Calculate distance from corner center
        let dx = px - corner_x;
        let dy = py - corner_y;
        let dist = (dx * dx + dy * dy).sqrt();
        
        // Fast analytical anti-aliasing
        if dist <= corner_radius - 0.5 {
            1.0
        } else if dist >= corner_radius + 0.5 {
            0.0
        } else {
            (corner_radius + 0.5 - dist).max(0.0).min(1.0)
        }
    }
}

/// Circle primitive
#[derive(Debug, Clone)]
pub struct Circle {
    center: Point,
    radius: FixedI32<U16>,
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
            radius: FixedI32::<U16>::from_num(radius.max(0.0)),
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
    
    /// Set anti-aliasing quality (alias for anti_aliasing)
    #[inline]
    pub fn aa(mut self, aa: AntiAliasing) -> Self {
        self.anti_aliasing(aa)
    }
}

impl Drawable for Circle {
    fn draw(self, canvas: &mut Canvas2D) {
        let radius_f = self.radius.to_num::<f32>();
        
        if let Some(fill) = &self.fill {
            self.draw_filled_circle(canvas, fill, radius_f);
        }
        
        if let Some(stroke) = &self.stroke {
            self.draw_circle_stroke(canvas, stroke, radius_f);
        }
    }
}

impl Circle {
    /// Draw filled circle with analytical anti-aliasing
    fn draw_filled_circle(&self, canvas: &mut Canvas2D, fill: &Paint, radius: f32) {
        let bounds = (radius + 1.0) as i32;
        
        for y in (self.center.y - bounds)..=(self.center.y + bounds) {
            for x in (self.center.x - bounds)..=(self.center.x + bounds) {
                if x >= 0 && y >= 0 && x < canvas.width() as i32 && y < canvas.height() as i32 {
                    let coverage = self.calculate_circle_coverage(x, y, radius);
                    if coverage > 0.0 {
                        let mut color = fill.sample_at(Point::new(x, y));
                        color.a = (color.a as f32 * coverage) as u8;
                        if color.a > 0 {
                            canvas.set_pixel(x, y, color);
                        }
                    }
                }
            }
        }
    }
    
    /// Draw circle stroke with analytical anti-aliasing
    fn draw_circle_stroke(&self, canvas: &mut Canvas2D, stroke: &Stroke, radius: f32) {
        let stroke_width = stroke.effective_width().to_num::<f32>();
        let half_stroke = stroke_width / 2.0;
        let inner_radius = radius - half_stroke;
        let outer_radius = radius + half_stroke;
        let bounds = (outer_radius + 1.0) as i32;
        
        for y in (self.center.y - bounds)..=(self.center.y + bounds) {
            for x in (self.center.x - bounds)..=(self.center.x + bounds) {
                if x >= 0 && y >= 0 && x < canvas.width() as i32 && y < canvas.height() as i32 {
                    let coverage = self.calculate_ring_coverage(x, y, inner_radius, outer_radius);
                    if coverage > 0.0 {
                        let mut color = match &stroke.paint {
                            Paint::Solid(c) => *c,
                            _ => stroke.paint.sample_at(Point::new(x, y)),
                        };
                        color.a = (color.a as f32 * coverage) as u8;
                        if color.a > 0 {
                            canvas.set_pixel(x, y, color);
                        }
                    }
                }
            }
        }
    }
    
    /// Calculate coverage for filled circle using fast analytical method
    fn calculate_circle_coverage(&self, x: i32, y: i32, radius: f32) -> f32 {
        let dx = x as f32 - self.center.x as f32;
        let dy = y as f32 - self.center.y as f32;
        let dist = (dx * dx + dy * dy).sqrt();
        
        // Fast analytical anti-aliasing - single sample with smooth falloff
        if dist <= radius - 0.5 {
            1.0
        } else if dist >= radius + 0.5 {
            0.0
        } else {
            // Smooth linear falloff in the transition zone
            (radius + 0.5 - dist).max(0.0).min(1.0)
        }
    }
    
    /// Calculate coverage for circle ring (stroke) using fast analytical method
    fn calculate_ring_coverage(&self, x: i32, y: i32, inner_radius: f32, outer_radius: f32) -> f32 {
        let dx = x as f32 - self.center.x as f32;
        let dy = y as f32 - self.center.y as f32;
        let dist = (dx * dx + dy * dy).sqrt();
        
        // Fast analytical anti-aliasing for ring
        let outer_coverage = if dist <= outer_radius - 0.5 {
            1.0
        } else if dist >= outer_radius + 0.5 {
            0.0
        } else {
            (outer_radius + 0.5 - dist).max(0.0).min(1.0)
        };
        
        let inner_coverage = if inner_radius <= 0.5 {
            0.0
        } else if dist <= inner_radius - 0.5 {
            1.0
        } else if dist >= inner_radius + 0.5 {
            0.0
        } else {
            (inner_radius + 0.5 - dist).max(0.0).min(1.0)
        };
        
        outer_coverage - inner_coverage
    }
}

/// Line primitive
#[derive(Debug, Clone)]
pub struct Line {
    start: Point,
    end: Point,
    stroke: Option<Stroke>,
    anti_aliasing: AntiAliasing,
}

impl Line {
    /// Create new line
    #[inline]
    pub fn new(start: Point, end: Point) -> Self {
        Self {
            start,
            end,
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
    
    /// Set anti-aliasing quality (alias for anti_aliasing)
    #[inline]
    pub fn aa(mut self, aa: AntiAliasing) -> Self {
        self.anti_aliasing(aa)
    }
}

impl Drawable for Line {
    fn draw(self, canvas: &mut Canvas2D) {
        if let Some(stroke) = &self.stroke {
            let color = match &stroke.paint {
                Paint::Solid(c) => *c,
                _ => stroke.paint.sample_at(self.start),
            };
            
            let stroke_width = stroke.effective_width().to_num::<f32>().max(1.0);
            
            if stroke_width <= 1.5 {
                // Use Xiaolin Wu's anti-aliased line algorithm for thin lines
                self.draw_wu_line(canvas, color);
            } else {
                // Use thick line with proper anti-aliasing
                self.draw_thick_line(canvas, color, stroke_width);
            }
        }
    }
}

impl Line {
    /// Xiaolin Wu's anti-aliased line algorithm - industry standard for thin lines
    fn draw_wu_line(&self, canvas: &mut Canvas2D, color: Rgba8888) {
        let mut x0 = self.start.x as f32;
        let mut y0 = self.start.y as f32;
        let mut x1 = self.end.x as f32;
        let mut y1 = self.end.y as f32;
        
        let steep = (y1 - y0).abs() > (x1 - x0).abs();
        
        if steep {
            core::mem::swap(&mut x0, &mut y0);
            core::mem::swap(&mut x1, &mut y1);
        }
        
        if x0 > x1 {
            core::mem::swap(&mut x0, &mut x1);
            core::mem::swap(&mut y0, &mut y1);
        }
        
        let dx = x1 - x0;
        let dy = y1 - y0;
        let gradient = if dx == 0.0 { 1.0 } else { dy / dx };
        
        // Handle first endpoint
        let xend = x0.round();
        let yend = y0 + gradient * (xend - x0);
        let xgap = 1.0 - (x0 + 0.5).fract();
        let xpxl1 = xend as i32;
        let ypxl1 = yend.floor() as i32;
        
        if steep {
            self.plot_pixel(canvas, ypxl1, xpxl1, color, (1.0 - yend.fract()) * xgap);
            self.plot_pixel(canvas, ypxl1 + 1, xpxl1, color, yend.fract() * xgap);
        } else {
            self.plot_pixel(canvas, xpxl1, ypxl1, color, (1.0 - yend.fract()) * xgap);
            self.plot_pixel(canvas, xpxl1, ypxl1 + 1, color, yend.fract() * xgap);
        }
        
        let mut intery = yend + gradient;
        
        // Handle second endpoint
        let xend = x1.round();
        let yend = y1 + gradient * (xend - x1);
        let xgap = (x1 + 0.5).fract();
        let xpxl2 = xend as i32;
        let ypxl2 = yend.floor() as i32;
        
        if steep {
            self.plot_pixel(canvas, ypxl2, xpxl2, color, (1.0 - yend.fract()) * xgap);
            self.plot_pixel(canvas, ypxl2 + 1, xpxl2, color, yend.fract() * xgap);
        } else {
            self.plot_pixel(canvas, xpxl2, ypxl2, color, (1.0 - yend.fract()) * xgap);
            self.plot_pixel(canvas, xpxl2, ypxl2 + 1, color, yend.fract() * xgap);
        }
        
        // Main loop
        for x in (xpxl1 + 1)..xpxl2 {
            if steep {
                self.plot_pixel(canvas, intery.floor() as i32, x, color, 1.0 - intery.fract());
                self.plot_pixel(canvas, intery.floor() as i32 + 1, x, color, intery.fract());
            } else {
                self.plot_pixel(canvas, x, intery.floor() as i32, color, 1.0 - intery.fract());
                self.plot_pixel(canvas, x, intery.floor() as i32 + 1, color, intery.fract());
            }
            intery += gradient;
        }
    }
    
    /// Draw thick line with analytical anti-aliasing
    fn draw_thick_line(&self, canvas: &mut Canvas2D, color: Rgba8888, width: f32) {
        let half_width = width * 0.5;
        
        // Calculate line direction and normal
        let dx = (self.end.x - self.start.x) as f32;
        let dy = (self.end.y - self.start.y) as f32;
        let length = (dx * dx + dy * dy).sqrt();
        
        if length < 0.001 {
            // Point - draw as circle
            let center_x = self.start.x as f32;
            let center_y = self.start.y as f32;
            
            let min_x = (center_x - half_width - 1.0) as i32;
            let max_x = (center_x + half_width + 1.0) as i32;
            let min_y = (center_y - half_width - 1.0) as i32;
            let max_y = (center_y + half_width + 1.0) as i32;
            
            for y in min_y..=max_y {
                for x in min_x..=max_x {
                    let dist = ((x as f32 - center_x).powi(2) + (y as f32 - center_y).powi(2)).sqrt();
                    let coverage = self.calculate_circle_coverage(dist, half_width);
                    if coverage > 0.0 {
                        self.plot_pixel(canvas, x, y, color, coverage);
                    }
                }
            }
            return;
        }
        
        let nx = -dy / length; // Normal X
        let ny = dx / length;  // Normal Y
        
        // Calculate bounding box
        let min_x = (self.start.x.min(self.end.x) as f32 - half_width - 1.0) as i32;
        let max_x = (self.start.x.max(self.end.x) as f32 + half_width + 1.0) as i32;
        let min_y = (self.start.y.min(self.end.y) as f32 - half_width - 1.0) as i32;
        let max_y = (self.start.y.max(self.end.y) as f32 + half_width + 1.0) as i32;
        
        // Sample each pixel with 4x4 supersampling for high quality
        for y in min_y..=max_y {
            for x in min_x..=max_x {
                let coverage = self.calculate_line_coverage_supersampled(x, y, half_width, nx, ny, dx, dy);
                if coverage > 0.0 {
                    self.plot_pixel(canvas, x, y, color, coverage);
                }
            }
        }
    }
    
    /// Calculate line coverage using fast analytical method
    fn calculate_line_coverage_supersampled(&self, x: i32, y: i32, half_width: f32, nx: f32, ny: f32, dx: f32, dy: f32) -> f32 {
        let px = x as f32 + 0.5;
        let py = y as f32 + 0.5;
        
        let x0 = self.start.x as f32;
        let y0 = self.start.y as f32;
        let x1 = self.end.x as f32;
        let y1 = self.end.y as f32;
        
        // Vector from line start to point
        let dpx = px - x0;
        let dpy = py - y0;
        
        // Project point onto line direction
        let dot = dpx * dx + dpy * dy;
        let length_sq = dx * dx + dy * dy;
        
        // Check if projection is within line segment
        let distance = if dot < 0.0 {
            // Before start - distance to start point
            ((px - x0).powi(2) + (py - y0).powi(2)).sqrt()
        } else if dot > length_sq {
            // After end - distance to end point
            ((px - x1).powi(2) + (py - y1).powi(2)).sqrt()
        } else {
            // Within segment - distance to line
            (dpx * nx + dpy * ny).abs()
        };
        
        // Fast analytical anti-aliasing
        if distance <= half_width - 0.5 {
            1.0
        } else if distance >= half_width + 0.5 {
            0.0
        } else {
            (half_width + 0.5 - distance).max(0.0).min(1.0)
        }
    }
    
    /// Check if point is inside thick line using analytical geometry
    fn point_in_thick_line(&self, px: f32, py: f32, half_width: f32, nx: f32, ny: f32, dx: f32, dy: f32) -> bool {
        let x0 = self.start.x as f32;
        let y0 = self.start.y as f32;
        let x1 = self.end.x as f32;
        let y1 = self.end.y as f32;
        
        // Vector from line start to point
        let dpx = px - x0;
        let dpy = py - y0;
        
        // Project point onto line direction
        let dot = dpx * dx + dpy * dy;
        let length_sq = dx * dx + dy * dy;
        
        // Check if projection is within line segment
        if dot < 0.0 || dot > length_sq {
            // Outside line segment - check distance to endpoints
            let dist_start = ((px - x0).powi(2) + (py - y0).powi(2)).sqrt();
            let dist_end = ((px - x1).powi(2) + (py - y1).powi(2)).sqrt();
            return dist_start <= half_width || dist_end <= half_width;
        }
        
        // Distance from point to line
        let distance = (dpx * nx + dpy * ny).abs();
        distance <= half_width
    }
    
    /// Calculate circle coverage for round line caps
    fn calculate_circle_coverage(&self, distance: f32, radius: f32) -> f32 {
        if distance <= radius - 0.5 {
            1.0
        } else if distance >= radius + 0.5 {
            0.0
        } else {
            // Linear falloff in the transition zone
            (radius + 0.5 - distance).max(0.0).min(1.0)
        }
    }
    
    /// Plot pixel with alpha blending
    fn plot_pixel(&self, canvas: &mut Canvas2D, x: i32, y: i32, color: Rgba8888, coverage: f32) {
        if x >= 0 && y >= 0 && x < canvas.width() as i32 && y < canvas.height() as i32 && coverage > 0.0 {
            let alpha = (color.a as f32 * coverage.min(1.0)) as u8;
            if alpha > 0 {
                let aa_color = Rgba8888::new(color.r, color.g, color.b, alpha);
                canvas.set_pixel(x, y, aa_color);
            }
        }
    }
}

/// Arc primitive
#[derive(Debug, Clone)]
pub struct Arc {
    center: Point,
    radius: FixedI32<U16>,
    start_angle: FixedI32<U16>,
    end_angle: FixedI32<U16>,
    stroke: Option<Stroke>,
    anti_aliasing: AntiAliasing,
}

impl Arc {
    /// Create new arc
    #[inline]
    pub fn new(center: Point, radius: f32, start_angle: f32, end_angle: f32) -> Self {
        Self {
            center,
            radius: FixedI32::<U16>::from_num(radius.max(0.0)),
            start_angle: FixedI32::<U16>::from_num(start_angle),
            end_angle: FixedI32::<U16>::from_num(end_angle),
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
    
    /// Set anti-aliasing quality (alias for anti_aliasing)
    #[inline]
    pub fn aa(mut self, aa: AntiAliasing) -> Self {
        self.anti_aliasing(aa)
    }
}

impl Drawable for Arc {
    fn draw(self, canvas: &mut Canvas2D) {
        if let Some(stroke) = &self.stroke {
            let color = match &stroke.paint {
                Paint::Solid(c) => *c,
                _ => stroke.paint.sample_at(self.center),
            };
            
            let radius = self.radius.to_num::<f32>();
            let stroke_width = stroke.effective_width().to_num::<f32>();
            let half_stroke = stroke_width / 2.0;
            let inner_radius = radius - half_stroke;
            let outer_radius = radius + half_stroke;
            
            let start_rad = self.start_angle.to_num::<f32>();
            let end_rad = self.end_angle.to_num::<f32>();
            
            // Calculate normalized arc length
            let mut arc_length = end_rad - start_rad;
            if arc_length < 0.0 {
                arc_length += 2.0 * core::f32::consts::PI;
            }
            arc_length = arc_length.min(2.0 * core::f32::consts::PI);
            
            // Calculate bounding box
            let bounds = (outer_radius + 1.0) as i32;
            
            // Use supersampling anti-aliasing for smooth arcs
            for y in (self.center.y - bounds)..=(self.center.y + bounds) {
                for x in (self.center.x - bounds)..=(self.center.x + bounds) {
                    if x >= 0 && y >= 0 && x < canvas.width() as i32 && y < canvas.height() as i32 {
                        let coverage = self.calculate_arc_coverage(x, y, inner_radius, outer_radius, start_rad, arc_length);
                        if coverage > 0.0 {
                            let alpha = (color.a as f32 * coverage) as u8;
                            if alpha > 0 {
                                let aa_color = Rgba8888::new(color.r, color.g, color.b, alpha);
                                canvas.set_pixel(x, y, aa_color);
                            }
                        }
                    }
                }
            }
        }
    }
}

impl Arc {
    /// Calculate arc coverage using fast analytical method
    fn calculate_arc_coverage(&self, x: i32, y: i32, inner_radius: f32, outer_radius: f32, start_angle: f32, arc_length: f32) -> f32 {
        let px = x as f32 + 0.5;
        let py = y as f32 + 0.5;
        
        if !self.point_in_arc(px, py, inner_radius, outer_radius, start_angle, arc_length) {
            return 0.0;
        }
        
        let dx = px - self.center.x as f32;
        let dy = py - self.center.y as f32;
        let dist = (dx * dx + dy * dy).sqrt();
        
        // Fast analytical anti-aliasing for ring edges
        let outer_coverage = if dist <= outer_radius - 0.5 {
            1.0
        } else if dist >= outer_radius + 0.5 {
            0.0
        } else {
            (outer_radius + 0.5 - dist).max(0.0).min(1.0)
        };
        
        let inner_coverage = if inner_radius <= 0.5 {
            0.0
        } else if dist <= inner_radius - 0.5 {
            1.0
        } else if dist >= inner_radius + 0.5 {
            0.0
        } else {
            (inner_radius + 0.5 - dist).max(0.0).min(1.0)
        };
        
        outer_coverage - inner_coverage
    }
    
    /// Check if point is inside arc using analytical geometry
    fn point_in_arc(&self, px: f32, py: f32, inner_radius: f32, outer_radius: f32, start_angle: f32, arc_length: f32) -> bool {
        let dx = px - self.center.x as f32;
        let dy = py - self.center.y as f32;
        let dist = (dx * dx + dy * dy).sqrt();
        
        // Check if point is in the ring
        if dist < inner_radius || dist > outer_radius {
            return false;
        }
        
        // For full circle, always include
        if arc_length >= 2.0 * core::f32::consts::PI - 0.01 {
            return true;
        }
        
        // Check if point is within the arc angle range
        let angle = dy.atan2(dx);
        
        // Normalize angles to [0, 2π]
        let normalize_angle = |a: f32| -> f32 {
            let mut normalized = a % (2.0 * core::f32::consts::PI);
            if normalized < 0.0 {
                normalized += 2.0 * core::f32::consts::PI;
            }
            normalized
        };
        
        let norm_angle = normalize_angle(angle);
        let norm_start = normalize_angle(start_angle);
        let norm_end = normalize_angle(start_angle + arc_length);
        
        if norm_start <= norm_end {
            // Arc doesn't wrap around
            norm_angle >= norm_start && norm_angle <= norm_end
        } else {
            // Arc wraps around 0/2π
            norm_angle >= norm_start || norm_angle <= norm_end
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
    
    /// Set anti-aliasing quality (alias for anti_aliasing)
    #[inline]
    pub fn aa(mut self, aa: AntiAliasing) -> Self {
        self.anti_aliasing(aa)
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
        let step = FixedI32::<U16>::ONE / FixedI32::<U16>::from_num(samples_per_axis);
        
        for y in rect.top_left.y..=rect.bottom() {
            for x in rect.top_left.x..=rect.right() {
                let mut covered_samples = 0;
                
                // Multi-sample for anti-aliasing
                for sy in 0..samples_per_axis {
                    for sx in 0..samples_per_axis {
                        let sample_x = FixedI32::<U16>::from_num(x) + step * FixedI32::<U16>::from_num(sx) + step / FixedI32::<U16>::from_num(2);
                        let sample_y = FixedI32::<U16>::from_num(y) + step * FixedI32::<U16>::from_num(sy) + step / FixedI32::<U16>::from_num(2);
                        
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
            if px < left + corner_radii.top_left.to_num::<i32>() && py < top + corner_radii.top_left.to_num::<i32>() {
                let cx = left + corner_radii.top_left.to_num::<i32>();
                let cy = top + corner_radii.top_left.to_num::<i32>();
                let dist_sq = (px - cx) * (px - cx) + (py - cy) * (py - cy);
                let radius_sq = corner_radii.top_left.to_num::<i32>() * corner_radii.top_left.to_num::<i32>();
                return dist_sq <= radius_sq;
            }
            
            // Top-right corner
            if px > right - corner_radii.top_right.to_num::<i32>() && py < top + corner_radii.top_right.to_num::<i32>() {
                let cx = right - corner_radii.top_right.to_num::<i32>();
                let cy = top + corner_radii.top_right.to_num::<i32>();
                let dist_sq = (px - cx) * (px - cx) + (py - cy) * (py - cy);
                let radius_sq = corner_radii.top_right.to_num::<i32>() * corner_radii.top_right.to_num::<i32>();
                return dist_sq <= radius_sq;
            }
            
            // Bottom-right corner
            if px > right - corner_radii.bottom_right.to_num::<i32>() && py > bottom - corner_radii.bottom_right.to_num::<i32>() {
                let cx = right - corner_radii.bottom_right.to_num::<i32>();
                let cy = bottom - corner_radii.bottom_right.to_num::<i32>();
                let dist_sq = (px - cx) * (px - cx) + (py - cy) * (py - cy);
                let radius_sq = corner_radii.bottom_right.to_num::<i32>() * corner_radii.bottom_right.to_num::<i32>();
                return dist_sq <= radius_sq;
            }
            
            // Bottom-left corner
            if px < left + corner_radii.bottom_left.to_num::<i32>() && py > bottom - corner_radii.bottom_left.to_num::<i32>() {
                let cx = left + corner_radii.bottom_left.to_num::<i32>();
                let cy = bottom - corner_radii.bottom_left.to_num::<i32>();
                let dist_sq = (px - cx) * (px - cx) + (py - cy) * (py - cy);
                let radius_sq = corner_radii.bottom_left.to_num::<i32>() * corner_radii.bottom_left.to_num::<i32>();
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
        
        let half_width = stroke.effective_width() / FixedI32::<U16>::from_num(2);
        let width_int = half_width.to_num::<i32>();
        
        // Create outer rectangle that includes stroke width
        let outer_rect = RectGeometry::new(
            Point::new(rect.top_left.x - width_int, rect.top_left.y - width_int),
            Size::new(
                rect.size.width + (2 * width_int) as u32,
                rect.size.height + (2 * width_int) as u32,
            ),
        );
        
        // Expand corner radii for outer rectangle
        let outer_radii = CornerRadii::new(
            corner_radii.top_left.to_num::<f32>() + width_int as f32,
            corner_radii.top_right.to_num::<f32>() + width_int as f32,
            corner_radii.bottom_right.to_num::<f32>() + width_int as f32,
            corner_radii.bottom_left.to_num::<f32>() + width_int as f32,
        );
        
        for y in rect.top_left.y - width_int..=rect.bottom() + width_int {
            for x in rect.top_left.x - width_int..=rect.right() + width_int {
                let point = Point::new(x, y);
                
                // Check if point is on the stroke boundary
                let inside_outer = self.point_in_rounded_rect(point, &outer_rect, &outer_radii);
                
                // Create inner rectangle
                let inner_rect = RectGeometry::new(
                    Point::new(rect.top_left.x + width_int, rect.top_left.y + width_int),
                    Size::new(
                        rect.size.width.saturating_sub(2 * width_int as u32),
                        rect.size.height.saturating_sub(2 * width_int as u32),
                    ),
                );
                
                // Shrink corner radii for inner rectangle
                let inner_radii = CornerRadii::new(
                    (corner_radii.top_left.to_num::<f32>() - width_int as f32).max(0.0),
                    (corner_radii.top_right.to_num::<f32>() - width_int as f32).max(0.0),
                    (corner_radii.bottom_right.to_num::<f32>() - width_int as f32).max(0.0),
                    (corner_radii.bottom_left.to_num::<f32>() - width_int as f32).max(0.0),
                );
                
                let inside_inner = if inner_rect.size.width > 0 && inner_rect.size.height > 0 {
                    self.point_in_rounded_rect(point, &inner_rect, &inner_radii)
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
        radius: FixedI32<U16>,
        paint: &Paint,
        aa: AntiAliasing,
    ) {
        let radius_int = radius.to_num::<i32>();
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
        let step = FixedI32::<U16>::ONE / FixedI32::<U16>::from_num(samples_per_axis);
        
        for y in min_y..=max_y {
            for x in min_x..=max_x {
                let mut covered_samples = 0;
                
                for sy in 0..samples_per_axis {
                    for sx in 0..samples_per_axis {
                        let sample_x = FixedI32::<U16>::from_num(x) + step * FixedI32::<U16>::from_num(sx) + step / FixedI32::<U16>::from_num(2);
                        let sample_y = FixedI32::<U16>::from_num(y) + step * FixedI32::<U16>::from_num(sy) + step / FixedI32::<U16>::from_num(2);
                        
                        let dx = sample_x.to_num::<i32>() - center.x;
                        let dy = sample_y.to_num::<i32>() - center.y;
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
        radius: FixedI32<U16>,
        stroke: &Stroke,
    ) {
        // Use Bresenham's circle algorithm with stroke width
        let radius_int = radius.to_num::<i32>();
        let stroke_width = stroke.effective_width().to_num::<i32>();
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
        radius: FixedI32<U16>,
        start_angle: FixedI32<U16>,
        end_angle: FixedI32<U16>,
        stroke: &Stroke,
    ) {
        let radius_int = radius.to_num::<i32>();
        let stroke_width = stroke.effective_width().to_num::<i32>();
        let half_stroke = stroke_width / 2;
        
        let start_deg = (start_angle.to_num::<f32>() * 180.0 / core::f32::consts::PI) as i32;
        let end_deg = (end_angle.to_num::<f32>() * 180.0 / core::f32::consts::PI) as i32;
        
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
