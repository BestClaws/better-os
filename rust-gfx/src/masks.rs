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
    p1: Point,
    p2: Point,
    origo: Point,
    xy_steep: i32,  // X/(1024*Y) steepness (normalized dx relative to dy=1024)
    yx_steep: i32,  // Y/(1024*X) steepness (normalized dy relative to dx=1024)
    steep: i32,     // Helper: yx_steep for flat, xy_steep for steep
    spx: i32,       // Steepness per pixel (steep >> 2, absolute value)
    flat: bool,     // Is line near horizontal?
    inv: bool,      // Invert the mask
    side: LineSide,
}

impl LineMask {
    /// Create line mask from two points
    pub fn from_points(mut p1: Point, mut p2: Point, side: LineSide) -> Self {
        // Swap points so p1.y <= p2.y (LVGL does this for consistency)
        if p1.y > p2.y {
            core::mem::swap(&mut p1, &mut p2);
        }
        
        let dx = p2.x - p1.x;
        let dy = p2.y - p1.y;
        
        let flat = dx.abs() > dy.abs();
        
        // Calculate steepness values with LVGL's normalization
        // (1 << 20) / delta gives us a multiplier to normalize to 1024 scale
        let (xy_steep, yx_steep) = if flat {
            let xy_steep = if dy != 0 {
                let m = (1i64 << 20) / dy as i64;
                ((m * dx as i64) >> 10) as i32
            } else {
                0
            };
            let yx_steep = if dx != 0 {
                let m = (1i64 << 20) / dx as i64;
                ((m * dy as i64) >> 10) as i32
            } else {
                0
            };
            (xy_steep, yx_steep)
        } else {
            let xy_steep = if dy != 0 {
                let m = (1i64 << 20) / dy as i64;
                ((m * dx as i64) >> 10) as i32
            } else {
                0
            };
            let yx_steep = if dx != 0 {
                let m = (1i64 << 20) / dx as i64;
                ((m * dy as i64) >> 10) as i32
            } else {
                0
            };
            (xy_steep, yx_steep)
        };
        
        let steep = if flat { yx_steep } else { xy_steep };
        
        // Calculate inv based on side
        let inv = match side {
            LineSide::Left => false,
            LineSide::Right => true,
            LineSide::Top => steep > 0,
            LineSide::Bottom => steep <= 0,
        };
        
        // Steepness per pixel for antialiasing
        let spx = if steep < 0 { -steep >> 2 } else { steep >> 2 };
        
        Self {
            p1,
            p2,
            origo: p1,
            xy_steep,
            yx_steep,
            steep,
            spx,
            flat,
            inv,
            side,
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
    fn apply(&self, mask_buf: &mut [Opa], abs_x: i32, abs_y: i32, len: usize) -> MaskResult {
        if len == 0 {
            return MaskResult::FullCover;
        }
        
        // Make points relative to origo
        let x = abs_x - self.origo.x;
        let y = abs_y - self.origo.y;
        
        // Handle special cases (horizontal/vertical lines)
        if self.steep == 0 {
            if self.flat {
                // Horizontal line
                match self.side {
                    LineSide::Left | LineSide::Right => return MaskResult::FullCover,
                    LineSide::Top if y < 0 => return MaskResult::FullCover,
                    LineSide::Bottom if y > 0 => return MaskResult::FullCover,
                    _ => return MaskResult::Transparent,
                }
            } else {
                // Vertical line
                match self.side {
                    LineSide::Top | LineSide::Bottom => return MaskResult::FullCover,
                    LineSide::Right if x > 0 => return MaskResult::FullCover,
                    LineSide::Left => {
                        if x + (len as i32) < 0 {
                            return MaskResult::FullCover;
                        }
                        let k = -x;
                        if k < 0 {
                            return MaskResult::Transparent;
                        }
                        if k >= 0 && k < len as i32 {
                            for i in k as usize..len {
                                mask_buf[i] = 0;
                            }
                            return MaskResult::Changed;
                        }
                        return MaskResult::Changed;
                    }
                    LineSide::Right => {
                        if x + (len as i32) < 0 {
                            return MaskResult::Transparent;
                        }
                        let k = -x;
                        let k = k.max(0);
                        if k >= len as i32 {
                            return MaskResult::Transparent;
                        }
                        for i in 0..k.min(len as i32) as usize {
                            mask_buf[i] = 0;
                        }
                        return MaskResult::Changed;
                    }
                    _ => return MaskResult::FullCover,
                }
            }
        }
        
        if self.flat {
            self.apply_flat(mask_buf, x, y, len)
        } else {
            self.apply_steep(mask_buf, x, y, len)
        }
    }
}

impl LineMask {
    fn apply_flat(&self, mask_buf: &mut [Opa], x: i32, y: i32, len: usize) -> MaskResult {
        // Port of LVGL's line_mask_flat
        // Note: x/y are already relative to origo (subtracted in apply())
        
        // Check at the beginning of the mask
        let mut y_at_x = ((self.yx_steep as i64 * x as i64) >> 10) as i32;
        
        if self.yx_steep > 0 {
            if y_at_x > y {
                return if self.inv { MaskResult::FullCover } else { MaskResult::Transparent };
            }
        } else {
            if y_at_x < y {
                return if self.inv { MaskResult::FullCover } else { MaskResult::Transparent };
            }
        }
        
        // Check at the end of the mask  
        y_at_x = ((self.yx_steep as i64 * (x + len as i32) as i64) >> 10) as i32;
        if self.yx_steep > 0 {
            if y_at_x < y {
                return if self.inv { MaskResult::Transparent } else { MaskResult::FullCover };
            }
        } else {
            if y_at_x > y {
                return if self.inv { MaskResult::Transparent } else { MaskResult::FullCover };
            }
        }
        
        // Calculate x position where line crosses this y (with subpixel precision)
        let xe = if self.yx_steep > 0 {
            ((y as i64 * 256) * self.xy_steep as i64) >> 10
        } else {
            (((y + 1) as i64 * 256) * self.xy_steep as i64) >> 10
        };
        
        let xei = (xe >> 8) as i32;
        let xef = (xe & 0xFF) as i32;
        
        let mut px_h = if xef == 0 {
            255
        } else {
            255 - (((255 - xef) * self.spx) >> 8)
        };
        
        let mut k = xei - x;
        
        // First fractional pixel
        if xef != 0 {
            if k >= 0 && k < len as i32 {
                let mut m = 255 - (((255 - xef) * (255 - px_h)) >> 9);
                if self.inv {
                    m = 255 - m;
                }
                mask_buf[k as usize] = Self::mask_mix(mask_buf[k as usize], m as u8);
            }
            k += 1;
        }
        
        // Middle pixels
        while px_h > self.spx {
            if k >= 0 && k < len as i32 {
                let mut m = px_h - (self.spx >> 1);
                if self.inv {
                    m = 255 - m;
                }
                mask_buf[k as usize] = Self::mask_mix(mask_buf[k as usize], m as u8);
            }
            px_h -= self.spx;
            k += 1;
            if k >= len as i32 {
                break;
            }
        }
        
        // Final fractional pixel
        if k < len as i32 && k >= 0 {
            let x_inters = ((px_h as i64 * self.xy_steep as i64) >> 10) as i32;
            let mut m = ((x_inters * px_h) >> 9) as i32;
            if self.yx_steep < 0 {
                m = 255 - m;
            }
            if self.inv {
                m = 255 - m;
            }
            mask_buf[k as usize] = Self::mask_mix(mask_buf[k as usize], m as u8);
        }
        
        // Clear pixels on the appropriate side
        if self.inv {
            let k = xei - x;
            if k > len as i32 {
                return MaskResult::Transparent;
            }
            if k >= 0 {
                for i in 0..k as usize {
                    mask_buf[i] = 0;
                }
            }
        } else {
            k += 1;
            if k < 0 {
                return MaskResult::Transparent;
            }
            if k <= len as i32 {
                for i in k.max(0) as usize..len {
                    mask_buf[i] = 0;
                }
            }
        }
        
        MaskResult::Changed
    }
    
    fn apply_steep(&self, mask_buf: &mut [Opa], x: i32, y: i32, len: usize) -> MaskResult {
        // Port of LVGL's line_mask_steep
        // Note: x/y are already relative to origo (subtracted in apply())
        
        // At the beginning of the mask if the limit line is greater than the mask's y
        let mut x_at_y = ((self.xy_steep as i64 * y as i64) >> 10) as i32;
        if self.xy_steep > 0 {
            x_at_y += 1;
        }
        
        if x_at_y < x {
            return if self.inv { MaskResult::FullCover } else { MaskResult::Transparent };
        }
        
        // At the end of the mask if the limit line is smaller than the mask's y
        x_at_y = ((self.xy_steep as i64 * y as i64) >> 10) as i32;
        if x_at_y > x + len as i32 {
            return if self.inv { MaskResult::Transparent } else { MaskResult::FullCover };
        }
        
        // X start
        let xs = ((y * 256) as i64 * self.xy_steep as i64) >> 10;
        let mut xsi = (xs >> 8) as i32;
        let mut xsf = (xs & 0xFF) as i32;
        
        // X end
        let xe = (((y + 1) * 256) as i64 * self.xy_steep as i64) >> 10;
        let xei = (xe >> 8) as i32;
        let xef = (xe & 0xFF) as i32;
        
        let mut k = xsi - x;
        if xsi != xei && self.xy_steep < 0 && xsf == 0 {
            xsf = 0xFF;
            xsi = xei;
            k -= 1;
        }
        
        if xsi == xei {
            // Line crosses in single pixel
            if k >= 0 && k < len as i32 {
                let mut m = (xsf + xef) >> 1;
                if self.inv {
                    m = 255 - m;
                }
                mask_buf[k as usize] = Self::mask_mix(mask_buf[k as usize], m as u8);
            }
            k += 1;
            
            // Clear pixels
            if self.inv {
                let k = xsi - x;
                if k >= len as i32 {
                    return MaskResult::Transparent;
                }
                if k >= 0 {
                    for i in 0..k.min(len as i32) as usize {
                        mask_buf[i] = 0;
                    }
                }
            } else {
                let k = k.min(len as i32);
                if k == 0 {
                    return MaskResult::Transparent;
                }
                for i in k.max(0) as usize..len {
                    mask_buf[i] = 0;
                }
            }
        } else {
            // Line crosses multiple pixels - apply antialiasing
            if self.xy_steep < 0 {
                let y_inters = ((xsf * (-self.yx_steep)) >> 10) as i32;
                if k >= 0 && k < len as i32 {
                    let mut m = (y_inters * xsf) >> 9;
                    if self.inv {
                        m = 255 - m;
                    }
                    mask_buf[k as usize] = Self::mask_mix(mask_buf[k as usize], m as u8);
                }
                k -= 1;
                
                let x_inters = (((255 - y_inters) * (-self.xy_steep)) >> 10) as i32;
                if k >= 0 && k < len as i32 {
                    let mut m = 255 - (((255 - y_inters) * x_inters) >> 9);
                    if self.inv {
                        m = 255 - m;
                    }
                    mask_buf[k as usize] = Self::mask_mix(mask_buf[k as usize], m as u8);
                }
                k += 2;
                
                if self.inv {
                    let k = (xsi - x - 1).min(len as i32);
                    if k > 0 {
                        for i in 0..k as usize {
                            mask_buf[i] = 0;
                        }
                    }
                } else {
                    if k > len as i32 {
                        return MaskResult::FullCover;
                    }
                    if k >= 0 {
                        for i in k as usize..len {
                            mask_buf[i] = 0;
                        }
                    }
                }
            } else {
                let y_inters = (((255 - xsf) * self.yx_steep) >> 10) as i32;
                if k >= 0 && k < len as i32 {
                    let mut m = 255 - ((y_inters * (255 - xsf)) >> 9);
                    if self.inv {
                        m = 255 - m;
                    }
                    mask_buf[k as usize] = Self::mask_mix(mask_buf[k as usize], m as u8);
                }
                k += 1;
                
                let x_inters = (((255 - y_inters) * self.xy_steep) >> 10) as i32;
                if k >= 0 && k < len as i32 {
                    let mut m = ((255 - y_inters) * x_inters) >> 9;
                    if self.inv {
                        m = 255 - m;
                    }
                    mask_buf[k as usize] = Self::mask_mix(mask_buf[k as usize], m as u8);
                }
                k += 1;
                
                if self.inv {
                    let k = xsi - x;
                    if k > len as i32 {
                        return MaskResult::Transparent;
                    }
                    if k >= 0 {
                        for i in 0..k.min(len as i32) as usize {
                            mask_buf[i] = 0;
                        }
                    }
                } else {
                    let k = k.min(len as i32);
                    if k == 0 {
                        return MaskResult::Transparent;
                    }
                    if k > 0 {
                        for i in k as usize..len {
                            mask_buf[i] = 0;
                        }
                    }
                }
            }
        }
        
        MaskResult::Changed
    }
    
    #[inline]
    #[inline]
    fn mask_mix(current: u8, new: u8) -> u8 {
        // Early exits for performance and correctness (LVGL does this)
        if new >= 255 {
            return current;
        }
        if new <= 0 {
            return 0;
        }
        ((current as u32 * new as u32) / 255) as u8
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
