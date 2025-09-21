/// Advanced stroke system with anti-aliasing and line cap/join support
/// 
/// This module provides comprehensive stroke rendering with:
/// - Variable stroke widths with sub-pixel precision
/// - Anti-aliased edges for smooth appearance
/// - Line caps: butt, round, square
/// - Line joins: miter, round, bevel
/// - Dash patterns for decorative effects
/// - Optimized rasterization for performance

use super::types::{Point, Rgba8888, AntiAliasing};
use fixed::{FixedI32, types::extra::U16};
use super::paint::Paint;
use micromath::F32Ext;

/// Stroke defines how shape outlines are rendered
#[derive(Debug, Clone)]
pub struct Stroke {
    /// Stroke color or paint
    pub paint: Paint,
    /// Stroke width in pixels (sub-pixel precision)
    pub width: FixedI32<U16>,
    /// Line cap style
    pub cap: LineCap,
    /// Line join style
    pub join: LineJoin,
    /// Miter limit for miter joins
    pub miter_limit: FixedI32<U16>,
    /// Dash pattern (empty for solid lines)
    pub dash_pattern: heapless::Vec<FixedI32<U16>, 8>,
    /// Dash offset
    pub dash_offset: FixedI32<U16>,
    /// Anti-aliasing quality
    pub anti_aliasing: AntiAliasing,
}

impl Stroke {
    /// Create new stroke with color and width
    #[inline]
    pub fn new(color: Rgba8888, width: f32) -> Self {
        Self {
            paint: Paint::solid(color),
            width: FixedI32::<U16>::from_num(width.max(0.0)),
            cap: LineCap::Round,
            join: LineJoin::Round,
            miter_limit: FixedI32::<U16>::from_num(4.0),
            dash_pattern: heapless::Vec::new(),
            dash_offset: FixedI32::<U16>::ZERO,
            anti_aliasing: AntiAliasing::default(),
        }
    }
    
    /// Create stroke with paint
    #[inline]
    pub fn with_paint(paint: Paint, width: f32) -> Self {
        Self {
            paint,
            width: FixedI32::<U16>::from_num(width.max(0.0)),
            cap: LineCap::Round,
            join: LineJoin::Round,
            miter_limit: FixedI32::<U16>::from_num(4.0),
            dash_pattern: heapless::Vec::new(),
            dash_offset: FixedI32::<U16>::ZERO,
            anti_aliasing: AntiAliasing::default(),
        }
    }
    
    /// Set line cap style
    #[inline]
    pub fn with_cap(mut self, cap: LineCap) -> Self {
        self.cap = cap;
        self
    }
    
    /// Set line join style
    #[inline]
    pub fn with_join(mut self, join: LineJoin) -> Self {
        self.join = join;
        self
    }
    
    /// Set miter limit
    #[inline]
    pub fn with_miter_limit(mut self, limit: f32) -> Self {
        self.miter_limit = FixedI32::<U16>::from_num(limit.max(1.0));
        self
    }
    
    /// Set dash pattern
    #[inline]
    pub fn with_dash_pattern(mut self, pattern: &[f32]) -> Self {
        self.dash_pattern.clear();
        for &dash in pattern.iter().take(8) {
            let _ = self.dash_pattern.push(FixedI32::<U16>::from_num(dash.max(0.0)));
        }
        self
    }
    
    /// Set dash offset
    #[inline]
    pub fn with_dash_offset(mut self, offset: f32) -> Self {
        self.dash_offset = FixedI32::<U16>::from_num(offset);
        self
    }
    
    /// Set anti-aliasing quality
    #[inline]
    pub fn with_anti_aliasing(mut self, aa: AntiAliasing) -> Self {
        self.anti_aliasing = aa;
        self
    }
    
    /// Check if stroke has dash pattern
    #[inline(always)]
    pub fn is_dashed(&self) -> bool {
        !self.dash_pattern.is_empty()
    }
    
    /// Get effective stroke width (minimum 1 pixel for visibility)
    #[inline(always)]
    pub fn effective_width(&self) -> FixedI32<U16> {
        if self.width < FixedI32::<U16>::ONE {
            FixedI32::<U16>::ONE
        } else {
            self.width
        }
    }
}

/// Line cap styles for stroke endpoints
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LineCap {
    /// Flat cap at line end
    Butt,
    /// Rounded cap extending beyond line end
    Round,
    /// Square cap extending beyond line end
    Square,
}

/// Line join styles for stroke corners
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LineJoin {
    /// Sharp pointed join
    Miter,
    /// Rounded join
    Round,
    /// Flat angled join
    Bevel,
}

/// Stroke rasterizer for anti-aliased line rendering
pub struct StrokeRasterizer {
    /// Coverage buffer for anti-aliasing
    coverage_buffer: heapless::Vec<u8, 1024>,
}

impl StrokeRasterizer {
    /// Create new stroke rasterizer
    #[inline]
    pub fn new() -> Self {
        Self {
            coverage_buffer: heapless::Vec::new(),
        }
    }
    
    /// Rasterize line segment with anti-aliasing
    #[inline]
    pub fn rasterize_line<R: crate::libs::gfx::two_d::Rasterizer >(
        &mut self,
        raster: &mut R,
        start: Point,
        end: Point,
        stroke: &Stroke,
    ) {
        if stroke.width.to_bits() <= 0 {
            return;
        }
        
        match stroke.anti_aliasing {
            AntiAliasing::None => self.rasterize_line_aliased(raster, start, end, stroke),
            _ => self.rasterize_line_antialiased(raster, start, end, stroke),
        }
    }
    
    /// Rasterize line without anti-aliasing (fastest)
    #[inline]
    fn rasterize_line_aliased<R: crate::libs::gfx::two_d::Rasterizer >(
        &self,
        raster: &mut R,
        start: Point,
        end: Point,
        stroke: &Stroke,
    ) {
        let color = match &stroke.paint {
            Paint::Solid(c) => *c,
            _ => stroke.paint.sample_at(start), // Simplified for aliased rendering
        };
        
        let width = stroke.effective_width().to_num::<i32>().max(1);
        let half_width = width / 2;
        
        // Use Bresenham's line algorithm with thickness
        let dx = (end.x - start.x).abs();
        let dy = (end.y - start.y).abs();
        let sx = if start.x < end.x { 1 } else { -1 };
        let sy = if start.y < end.y { 1 } else { -1 };
        let mut err = dx - dy;
        
        let mut x = start.x;
        let mut y = start.y;
        
        loop {
            // Draw thick point
            for dy_offset in -half_width..=half_width {
                for dx_offset in -half_width..=half_width {
                    let px = x + dx_offset;
                    let py = y + dy_offset;
                    
                    if px >= 0 && py >= 0 && px < raster.width() as i32 && py < raster.height() as i32 {
                        raster.set_pixel(px, py, color);
                    }
                }
            }
            
            if x == end.x && y == end.y { break; }
            
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
    
    /// Rasterize line with anti-aliasing
    #[inline]
    fn rasterize_line_antialiased<R: crate::libs::gfx::two_d::Rasterizer >(
        &mut self,
        raster: &mut R,
        start: Point,
        end: Point,
        stroke: &Stroke,
    ) {
        let width = stroke.effective_width();
        let half_width = width / FixedI32::<U16>::from_num(2);
        
        // Calculate line direction and normal
        let dx = end.x - start.x;
        let dy = end.y - start.y;
        let length_sq = dx * dx + dy * dy;
        
        if length_sq == 0 {
            // Point - draw as circle
            self.rasterize_point_antialiased(raster, start, stroke);
            return;
        }
        
        let length = FixedI32::<U16>::from_num((length_sq as f32).sqrt());
        let nx = FixedI32::<U16>::from_num(-dy) / length; // Normal X
        let ny = FixedI32::<U16>::from_num(dx) / length;  // Normal Y
        
        // Calculate bounding box
        let min_x = (start.x.min(end.x) - half_width.to_num::<i32>() - 1).max(0);
        let max_x = (start.x.max(end.x) + half_width.to_num::<i32>() + 1).min(raster.width() as i32 - 1);
        let min_y = (start.y.min(end.y) - half_width.to_num::<i32>() - 1).max(0);
        let max_y = (start.y.max(end.y) + half_width.to_num::<i32>() + 1).min(raster.height() as i32 - 1);
        
        // Sample each pixel in bounding box
        for y in min_y..=max_y {
            for x in min_x..=max_x {
                let coverage = self.calculate_line_coverage(
                    Point::new(x, y),
                    start,
                    end,
                    half_width,
                    stroke.anti_aliasing,
                );
                
                if coverage > 0 {
                    let color = stroke.paint.sample_at(Point::new(x, y));
                    raster.blend_pixel(x, y, color, coverage);
                }
            }
        }
    }
    
    /// Calculate coverage for a pixel relative to a line
    #[inline]
    fn calculate_line_coverage(
        &self,
        pixel: Point,
        line_start: Point,
        line_end: Point,
        half_width: FixedI32<U16>,
        aa_quality: AntiAliasing,
    ) -> u8 {
        let samples = match aa_quality {
            AntiAliasing::None => 1,
            AntiAliasing::Low => 4,
            AntiAliasing::Medium => 16,
            AntiAliasing::High => 64,
        };
        
        let mut covered_samples = 0;
        let samples_per_axis = (samples as f32).sqrt() as i32;
        let step = FixedI32::<U16>::ONE / FixedI32::<U16>::from_num(samples_per_axis);
        
        for sy in 0..samples_per_axis {
            for sx in 0..samples_per_axis {
                let sample_x = FixedI32::<U16>::from_num(pixel.x) + step * FixedI32::<U16>::from_num(sx) + step / FixedI32::<U16>::from_num(2);
                let sample_y = FixedI32::<U16>::from_num(pixel.y) + step * FixedI32::<U16>::from_num(sy) + step / FixedI32::<U16>::from_num(2);
                
                let distance = self.point_to_line_distance(
                    Point::from_fixed(sample_x, sample_y),
                    line_start,
                    line_end,
                );
                
                if distance <= half_width {
                    covered_samples += 1;
                }
            }
        }
        
        ((covered_samples * 255) / samples) as u8
    }
    
    /// Calculate distance from point to line segment
    #[inline]
    fn point_to_line_distance(&self, point: Point, line_start: Point, line_end: Point) -> FixedI32<U16> {
        let dx = line_end.x - line_start.x;
        let dy = line_end.y - line_start.y;
        let length_sq = dx * dx + dy * dy;
        
        if length_sq == 0 {
            // Line is a point
            let px_diff = point.x - line_start.x;
            let py_diff = point.y - line_start.y;
            return FixedI32::<U16>::from_num((px_diff * px_diff + py_diff * py_diff) as f32).sqrt();
        }
        
        // Project point onto line
        let px = point.x - line_start.x;
        let py = point.y - line_start.y;
        let dot = px * dx + py * dy;
        let t = (dot as f32 / length_sq as f32).max(0.0).min(1.0);
        
        // Find closest point on line segment
        let closest_x = line_start.x + (dx as f32 * t) as i32;
        let closest_y = line_start.y + (dy as f32 * t) as i32;
        
        // Calculate distance
        let dist_x = point.x - closest_x;
        let dist_y = point.y - closest_y;
        FixedI32::<U16>::from_num((dist_x * dist_x + dist_y * dist_y) as f32).sqrt()
    }
    
    /// Rasterize point with anti-aliasing (for zero-length lines)
    #[inline]
    fn rasterize_point_antialiased<R: crate::libs::gfx::two_d::Rasterizer >(
        &mut self,
        raster: &mut R,
        center: Point,
        stroke: &Stroke,
    ) {
        let radius = stroke.effective_width() / FixedI32::<U16>::from_num(2);
        let radius_int = radius.to_num::<i32>();
        
        let min_x = (center.x - radius_int - 1).max(0);
        let max_x = (center.x + radius_int + 1).min(raster.width() as i32 - 1);
        let min_y = (center.y - radius_int - 1).max(0);
        let max_y = (center.y + radius_int + 1).min(raster.height() as i32 - 1);
        
        for y in min_y..=max_y {
            for x in min_x..=max_x {
                let distance = FixedI32::<U16>::from_num(center.distance_squared(Point::new(x, y)) as f32).sqrt();
                
                if distance <= radius {
                    let coverage = if distance >= radius - FixedI32::<U16>::ONE {
                        // Anti-aliased edge
                        let edge_coverage = (radius - distance) / FixedI32::<U16>::ONE;
                        (edge_coverage.to_num::<f32>() * 255.0) as u8
                    } else {
                        255
                    };
                    
                    if coverage > 0 {
                        let color = stroke.paint.sample_at(Point::new(x, y));
                        raster.blend_pixel(x, y, color, coverage);
                    }
                }
            }
        }
    }
    
    /// Rasterize line cap
    #[inline]
    pub fn rasterize_cap<R: crate::libs::gfx::two_d::Rasterizer >(
        &mut self,
        raster: &mut R,
        point: Point,
        direction: Point, // Normalized direction vector
        stroke: &Stroke,
    ) {
        match stroke.cap {
            LineCap::Butt => {
                // No additional rendering needed
            }
            LineCap::Round => {
                self.rasterize_point_antialiased(raster, point, stroke);
            }
            LineCap::Square => {
                let half_width = stroke.effective_width() / FixedI32::<U16>::from_num(2);
                let extension = half_width;
                
                // Calculate square cap corners
                let normal_x = -direction.y;
                let normal_y = direction.x;
                
                let corner1 = Point::new(
                    point.x + (normal_x as f32 * half_width.to_num::<f32>()) as i32 + (direction.x as f32 * extension.to_num::<f32>()) as i32,
                    point.y + (normal_y as f32 * half_width.to_num::<f32>()) as i32 + (direction.y as f32 * extension.to_num::<f32>()) as i32,
                );
                let corner2 = Point::new(
                    point.x - (normal_x as f32 * half_width.to_num::<f32>()) as i32 + (direction.x as f32 * extension.to_num::<f32>()) as i32,
                    point.y - (normal_y as f32 * half_width.to_num::<f32>()) as i32 + (direction.y as f32 * extension.to_num::<f32>()) as i32,
                );
                
                // Render square cap as rectangle
                self.rasterize_line_antialiased(raster, corner1, corner2, stroke);
            }
        }
    }
}

impl Default for StrokeRasterizer {
    fn default() -> Self {
        Self::new()
    }
}
