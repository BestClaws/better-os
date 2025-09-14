//! Utility helpers for rectangle math used by the drawing surface.
use crate::libs::gfx::two_d::{Rect, Point, Size};

#[inline(always)]
pub fn clip_rect(area: &Rect, width: u32, height: u32) -> (u32, u32, u32, u32) {
    let x0 = area.top_left.x.max(0) as u32;
    let y0 = area.top_left.y.max(0) as u32;
    let x1 = (area.top_left.x as u32 + area.size.width).min(width);
    let y1 = (area.top_left.y as u32 + area.size.height).min(height);
    (x0, y0, x1, y1)
}

#[inline(always)]
pub fn union_rect(r1: Rect, r2: Rect) -> Rect {
    let left = r1.top_left.x.min(r2.top_left.x);
    let top = r1.top_left.y.min(r2.top_left.y);

    let right = (r1.top_left.x + r1.size.width as i32).max(r2.top_left.x + r2.size.width as i32);
    let bottom = (r1.top_left.y + r1.size.height as i32).max(r2.top_left.y + r2.size.height as i32);

    Rect::with_corners(Point::new(left, top), Point::new(right - 1, bottom - 1))
}

#[inline(always)]
pub fn intersects_or_touches(a: &Rect, b: &Rect) -> bool {
    let ax0 = a.top_left.x;
    let ay0 = a.top_left.y;
    let ax1 = a.top_left.x + a.size.width as i32; // exclusive
    let ay1 = a.top_left.y + a.size.height as i32; // exclusive

    let bx0 = b.top_left.x;
    let by0 = b.top_left.y;
    let bx1 = b.top_left.x + b.size.width as i32; // exclusive
    let by1 = b.top_left.y + b.size.height as i32; // exclusive

    // Allow touching by expanding B by 1 pixel in each direction
    let bx0t = bx0 - 1;
    let by0t = by0 - 1;
    let bx1t = bx1 + 1;
    let by1t = by1 + 1;

    !(ax1 <= bx0t || ax0 >= bx1t || ay1 <= by0t || ay0 >= by1t)
}


