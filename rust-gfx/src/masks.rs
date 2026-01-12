/// Mask system matching LVGL's software rendering masks
/// Masks are used to constrain drawing to specific regions with antialiasing

use alloc::vec;
use alloc::vec::Vec;
use crate::types::*;

/// Result of applying a mask to a pixel row
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MaskResult {
    /// Entire row is transparent (all zeros)
    Transparent,
    /// Entire row is fully opaque (all 255)
    FullCover,
    /// Mask buffer was modified with per-pixel opacity
    Changed,
}

/// Trait for all mask types
pub trait Mask {
    /// Apply mask to a horizontal line of pixels
    /// mask_buf: buffer to write mask values (0-255 opacity), must be initialized to 255
    /// x, y: absolute coordinates of the line start
    /// len: number of pixels in the line
    /// Returns: MaskResult indicating if the buffer was modified
    fn apply(&self, mask_buf: &mut [Opa], x: i32, y: i32, len: usize) -> MaskResult;
}

/// Line side for line masks
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LineSide {
    Left,
    Right,
    Top,
    Bottom,
}

/// Line mask - keeps pixels on one side of a line
#[derive(Debug, Clone)]
pub struct LineMask {
    origo: Point,
    xy_steep: i32,  // X/(1024*Y) steepness
    yx_steep: i32,  // Y/(1024*X) steepness  
    steep: i32,     // Helper: yx_steep for flat, xy_steep for steep
    spx: i32,       // Steepness in 1px (0-255 range) for flat lines
    flat: bool,     // Is line near horizontal?
    inv: bool,      // Invert the mask
}

impl LineMask {
    /// Create line mask from two points
    pub fn from_points(p1: Point, p2: Point, side: LineSide) -> Self {
        let dx = p2.x - p1.x;
        let dy = p2.y - p1.y;
        
        let flat = dx.abs() > dy.abs();
        
        // Calculate steepness values (fixed point with 1024 = 1.0)
        let xy_steep = if dy != 0 {
            (dx * 1024) / dy
        } else {
            i32::MAX
        };
        
        let yx_steep = if dx != 0 {
            (dy * 1024) / dx
        } else {
            i32::MAX
        };
        
        let steep = if flat { yx_steep } else { xy_steep };
        
        // Calculate single-pixel steepness for flat lines (0-255 range)
        let spx = if flat && dx != 0 {
            ((dy.abs() << 8) / dx.abs()).min(255)
        } else {
            0
        };
        
        // Determine if we need to invert based on side
        let inv = match side {
            LineSide::Right | LineSide::Bottom => true,
            LineSide::Left | LineSide::Top => false,
        };
        
        Self {
            origo: p1,
            xy_steep,
            yx_steep,
            steep,
            spx,
            flat,
            inv,
        }
    }
    
    /// Create line mask from point and angle (in degrees)
    pub fn from_angle(px: i32, py: i32, angle: i32, side: LineSide) -> Self {
        // Convert angle to second point
        // Angle 0 is right, 90 is down
        // Use integer approximation to avoid floating point
        let len = 100;
        
        // Use lookup table or integer trig approximation
        // For simplicity, use basic approximation
        let angle_mod = angle % 360;
        let (dx, dy) = match angle_mod {
            0 => (len, 0),
            90 => (0, len),
            180 => (-len, 0),
            270 => (0, -len),
            _ => {
                // Approximate with small angle steps
                // This is a simple approximation - LVGL uses proper sin/cos
                let quadrant = angle_mod / 90;
                let remainder = angle_mod % 90;
                match quadrant {
                    0 => (len - remainder, remainder),
                    1 => (-remainder, len - remainder),
                    2 => (-(len - remainder), -remainder),
                    _ => (remainder, -(len - remainder)),
                }
            }
        };
        
        Self::from_points(
            Point::new(px, py),
            Point::new(px + dx, py + dy),
            side
        )
    }
}

impl Mask for LineMask {
    fn apply(&self, mask_buf: &mut [Opa], x: i32, y: i32, len: usize) -> MaskResult {
        if len == 0 {
            return MaskResult::FullCover;
        }
        
        let mut changed = false;
        
        if self.flat {
            // Horizontal-ish line
            let y_diff = y - self.origo.y;
            let x_diff = x - self.origo.x;
            
            for i in 0..len {
                let xi = x + i as i32;
                let xi_diff = xi - self.origo.x;
                
                // Calculate where the line crosses this x coordinate
                let y_at_x = if self.yx_steep != i32::MAX {
                    self.origo.y + (xi_diff * self.yx_steep) / 1024
                } else {
                    self.origo.y
                };
                
                let diff = y - y_at_x;
                let opa = if self.inv {
                    if diff >= 2 {
                        255
                    } else if diff >= 1 {
                        255 - ((self.spx * (-diff + 1)) / 2) as u8
                    } else if diff <= -2 {
                        0
                    } else if diff <= -1 {
                        ((self.spx * (diff + 1)) / 2) as u8
                    } else {
                        128
                    }
                } else {
                    if diff <= -2 {
                        255
                    } else if diff <= -1 {
                        255 - ((self.spx * (diff + 1)) / 2) as u8
                    } else if diff >= 2 {
                        0
                    } else if diff >= 1 {
                        ((self.spx * (-diff + 1)) / 2) as u8
                    } else {
                        128
                    }
                };
                
                if opa < 255 {
                    changed = true;
                    mask_buf[i] = ((mask_buf[i] as u32 * opa as u32) / 255) as Opa;
                }
            }
        } else {
            // Vertical-ish line
            for i in 0..len {
                let xi = x + i as i32;
                let xi_diff = xi - self.origo.x;
                
                // Calculate where the line crosses this x coordinate
                let y_at_x = if self.xy_steep != i32::MAX {
                    self.origo.y + (xi_diff * self.xy_steep) / 1024
                } else {
                    i32::MAX
                };
                
                let diff = y - y_at_x;
                let opa = if self.inv {
                    if diff > 0 { 255 } else { 0 }
                } else {
                    if diff < 0 { 255 } else { 0 }
                };
                
                if opa < 255 {
                    changed = true;
                    mask_buf[i] = ((mask_buf[i] as u32 * opa as u32) / 255) as Opa;
                }
            }
        }
        
        if changed {
            MaskResult::Changed
        } else {
            MaskResult::FullCover
        }
    }
}

/// Angle mask - keeps pixels within an angular range
#[derive(Debug, Clone)]
pub struct AngleMask {
    vertex: Point,
    start_angle: i32,
    end_angle: i32,
    delta_deg: u16,
    start_line: LineMask,
    end_line: LineMask,
}

impl AngleMask {
    pub fn new(vertex_x: i32, vertex_y: i32, start_angle: i32, end_angle: i32) -> Self {
        // Constrain angles to 0-359
        let start_angle = start_angle.max(0).min(359);
        let end_angle = end_angle.max(0).min(359);
        
        let delta_deg = if end_angle < start_angle {
            (360 - start_angle + end_angle) as u16
        } else {
            (end_angle - start_angle) as u16
        };
        
        // Determine line sides based on angles
        let start_side = if start_angle >= 0 && start_angle < 180 {
            LineSide::Left
        } else {
            LineSide::Right
        };
        
        let end_side = if end_angle >= 0 && end_angle < 180 {
            LineSide::Right
        } else if end_angle >= 180 && end_angle < 360 {
            LineSide::Left
        } else {
            LineSide::Right
        };
        
        Self {
            vertex: Point::new(vertex_x, vertex_y),
            start_angle,
            end_angle,
            delta_deg,
            start_line: LineMask::from_angle(vertex_x, vertex_y, start_angle, start_side),
            end_line: LineMask::from_angle(vertex_x, vertex_y, end_angle, end_side),
        }
    }
}

impl Mask for AngleMask {
    fn apply(&self, mask_buf: &mut [Opa], x: i32, y: i32, len: usize) -> MaskResult {
        // Apply both line masks
        let res1 = self.start_line.apply(mask_buf, x, y, len);
        let res2 = self.end_line.apply(mask_buf, x, y, len);
        
        if res1 == MaskResult::Transparent || res2 == MaskResult::Transparent {
            MaskResult::Transparent
        } else if res1 == MaskResult::Changed || res2 == MaskResult::Changed {
            MaskResult::Changed
        } else {
            MaskResult::FullCover
        }
    }
}

/// Radius mask - keeps pixels inside/outside a rounded rectangle
#[derive(Debug, Clone)]
pub struct RadiusMask {
    rect: Area,
    radius: i32,
    outer: bool,  // true: keep outside, false: keep inside
}

impl RadiusMask {
    pub fn new(rect: Area, radius: i32, outer: bool) -> Self {
        Self {
            rect,
            radius,
            outer,
        }
    }
}

impl Mask for RadiusMask {
    fn apply(&self, mask_buf: &mut [Opa], x: i32, y: i32, len: usize) -> MaskResult {
        // Simplified radius mask - full implementation would use circle cache
        let mut changed = false;
        
        for i in 0..len {
            let xi = x + i as i32;
            
            // Check if point is in corner region
            let in_corner = (xi < self.rect.x1 + self.radius && y < self.rect.y1 + self.radius) ||
                           (xi > self.rect.x2 - self.radius && y < self.rect.y1 + self.radius) ||
                           (xi < self.rect.x1 + self.radius && y > self.rect.y2 - self.radius) ||
                           (xi > self.rect.x2 - self.radius && y > self.rect.y2 - self.radius);
            
            if in_corner {
                // Calculate distance to corner center
                let cx = if xi < self.rect.x1 + self.radius {
                    self.rect.x1 + self.radius
                } else {
                    self.rect.x2 - self.radius
                };
                
                let cy = if y < self.rect.y1 + self.radius {
                    self.rect.y1 + self.radius
                } else {
                    self.rect.y2 - self.radius
                };
                
                let dx = xi - cx;
                let dy = y - cy;
                let dist_sq = dx * dx + dy * dy;
                let r_sq = self.radius * self.radius;
                
                let opa = if self.outer {
                    // Keep outside
                    if dist_sq > r_sq { 255 } else { 0 }
                } else {
                    // Keep inside
                    if dist_sq < r_sq { 255 } else { 0 }
                };
                
                if opa < 255 {
                    changed = true;
                    mask_buf[i] = ((mask_buf[i] as u32 * opa as u32) / 255) as Opa;
                }
            }
        }
        
        if changed {
            MaskResult::Changed
        } else {
            MaskResult::FullCover
        }
    }
}

/// Apply multiple masks to a buffer
/// Returns MaskResult indicating the combined result
pub fn apply_masks(masks: &[&dyn Mask], mask_buf: &mut [Opa], x: i32, y: i32, len: usize) -> MaskResult {
    if masks.is_empty() {
        return MaskResult::FullCover;
    }
    
    let mut result = MaskResult::FullCover;
    
    for mask in masks {
        let res = mask.apply(mask_buf, x, y, len);
        if res == MaskResult::Transparent {
            return MaskResult::Transparent;
        }
        if res == MaskResult::Changed {
            result = MaskResult::Changed;
        }
    }
    
    result
}
