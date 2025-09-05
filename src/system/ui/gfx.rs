//! Minimal, space-grade 2D raster graphics framework for embedded targets.
//!
//! Features:
//! - Core geometry types: `Point`, `Size`, `Rect`
//! - Color model: `Rgb565`
//! - Pixel blending with integer math (pre-multiplied alpha style over operator)
//! - Anti-aliased drawing of lines (Xiaolin Wu), circles, arcs
//! - Primitives: lines, rectangles, rounded rectangles, circles, arcs
//! - Gradient fills: linear and radial
//! - Extensible traits for future primitives

#![allow(clippy::many_single_char_names)]

use core::cmp::{max, min};
use micromath::F32Ext;

// ========================= Geometry =========================

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Point {
    pub x: i32,
    pub y: i32,
}

impl Point {
    pub const fn new(x: i32, y: i32) -> Self { Self { x, y } }
    pub const fn zero() -> Self { Self { x: 0, y: 0 } }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Size {
    pub width: u32,
    pub height: u32,
}

impl Size {
    pub const fn new(width: u32, height: u32) -> Self { Self { width, height } }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Rect {
    pub top_left: Point,
    pub size: Size,
}

impl Rect {
    pub const fn new(top_left: Point, size: Size) -> Self { Self { top_left, size } }

    pub fn with_corners(a: Point, b: Point) -> Self {
        let left = min(a.x, b.x);
        let top = min(a.y, b.y);
        let right = max(a.x, b.x);
        let bottom = max(a.y, b.y);
        Self {
            top_left: Point::new(left, top),
            size: Size::new((right - left + 1) as u32, (bottom - top + 1) as u32),
        }
    }

    pub fn right(&self) -> i32 { self.top_left.x + self.size.width as i32 - 1 }
    pub fn bottom(&self) -> i32 { self.top_left.y + self.size.height as i32 - 1 }

    pub fn intersects(&self, other: &Rect) -> bool {
        !(self.right() < other.top_left.x
            || other.right() < self.top_left.x
            || self.bottom() < other.top_left.y
            || other.bottom() < self.top_left.y)
    }

    pub fn intersection(&self, other: &Rect) -> Option<Rect> {
        if !self.intersects(other) { return None; }
        let left = max(self.top_left.x, other.top_left.x);
        let top = max(self.top_left.y, other.top_left.y);
        let right = min(self.right(), other.right());
        let bottom = min(self.bottom(), other.bottom());
        Some(Rect::with_corners(Point::new(left, top), Point::new(right, bottom)))
    }

    pub fn union(&self, other: &Rect) -> Rect {
        let left = min(self.top_left.x, other.top_left.x);
        let top = min(self.top_left.y, other.top_left.y);
        let right = max(self.right(), other.right());
        let bottom = max(self.bottom(), other.bottom());
        Rect::with_corners(Point::new(left, top), Point::new(right, bottom))
    }
}

// ========================= Color =========================

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Rgb565(pub u16);

impl Rgb565 {
    pub const BLACK: Self = Self(0x0000);
    pub const WHITE: Self = Self(0xFFFF);

    pub const fn from_rgb(r8: u8, g8: u8, b8: u8) -> Self {
        let r5: u16 = ((r8 as u16) >> 3) & 0x1F;
        let g6: u16 = ((g8 as u16) >> 2) & 0x3F;
        let b5: u16 = ((b8 as u16) >> 3) & 0x1F;
        let packed: u16 = (r5 << 11) | (g6 << 5) | b5;
        Self(packed)
    }

    pub const fn into_storage(self) -> u16 { self.0 }

    pub fn blend_over(self, bg: Rgb565, alpha_u8: u8) -> Rgb565 {
        // Integer alpha blend in 5/6/5 space
        let a = alpha_u8 as u32; // 0..255
        if a == 255 { return self; }
        if a == 0 { return bg; }

        let sr = ((self.0 >> 11) & 0x1F) as u32;
        let sg = ((self.0 >> 5) & 0x3F) as u32;
        let sb = (self.0 & 0x1F) as u32;

        let br = ((bg.0 >> 11) & 0x1F) as u32;
        let bgc = ((bg.0 >> 5) & 0x3F) as u32;
        let bb = (bg.0 & 0x1F) as u32;

        let r = (sr * a + br * (255 - a) + 127) / 255;
        let g = (sg * a + bgc * (255 - a) + 127) / 255;
        let b = (sb * a + bb * (255 - a) + 127) / 255;

        let packed: u16 = (((r & 0x1F) as u16) << 11)
            | (((g & 0x3F) as u16) << 5)
            | ((b & 0x1F) as u16);
        Rgb565(packed)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Rgba8888 {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl Rgba8888 {
    pub const fn new(r: u8, g: u8, b: u8, a: u8) -> Self { Self { r, g, b, a } }
    pub const fn opaque(r: u8, g: u8, b: u8) -> Self { Self { r, g, b, a: 255 } }
    pub const fn transparent() -> Self { Self { r: 0, g: 0, b: 0, a: 0 } }
    pub fn to_rgb565(self) -> Rgb565 { Rgb565::from_rgb(self.r, self.g, self.b) }
}

// ========================= Rasterizer Trait =========================

pub trait Rasterizer {
    fn width(&self) -> u32;
    fn height(&self) -> u32;
    fn set_pixel(&mut self, x: i32, y: i32, color: Rgb565);
    fn blend_pixel(&mut self, x: i32, y: i32, color: Rgb565, alpha: u8);
}

// ========================= Gradients =========================

#[derive(Clone, Copy, Debug)]
pub struct LinearGradient {
    pub start: Point,
    pub end: Point,
    pub start_color: Rgb565,
    pub end_color: Rgb565,
}

impl LinearGradient {
    pub fn sample(&self, p: Point) -> Rgb565 {
        let dx = (self.end.x - self.start.x) as i64;
        let dy = (self.end.y - self.start.y) as i64;
        let len2 = (dx * dx + dy * dy).max(1);
        let t_num = ((p.x - self.start.x) as i64 * dx + (p.y - self.start.y) as i64 * dy).clamp(0, len2);
        let t = (t_num as f32) / (len2 as f32);
        lerp_rgb565(self.start_color, self.end_color, t)
    }
}

#[derive(Clone, Copy, Debug)]
pub struct RadialGradient {
    pub center: Point,
    pub radius: u32,
    pub inner_color: Rgb565,
    pub outer_color: Rgb565,
}

impl RadialGradient {
    pub fn sample(&self, p: Point) -> Rgb565 {
        let dx = p.x - self.center.x;
        let dy = p.y - self.center.y;
        let d2 = (dx as i64 * dx as i64 + dy as i64 * dy as i64) as f32;
        let r = (self.radius as f32).max(1.0);
        let t = (d2.sqrt() / r).clamp(0.0, 1.0);
        lerp_rgb565(self.inner_color, self.outer_color, t)
    }
}

fn lerp_rgb565(a: Rgb565, b: Rgb565, t: f32) -> Rgb565 {
    let at = (t * 255.0).clamp(0.0, 255.0) as u8;
    b.blend_over(a, at)
}

// ========================= Drawing Algorithms =========================

pub fn clear(r: &mut dyn Rasterizer, color: Rgb565) {
    for y in 0..r.height() as i32 {
        for x in 0..r.width() as i32 {
            r.set_pixel(x, y, color);
        }
    }
}

pub fn fill_rect(r: &mut dyn Rasterizer, rect: Rect, color: Rgb565) {
    let clip = Rect::new(Point::zero(), Size::new(r.width(), r.height()));
    if let Some(rc) = rect.intersection(&clip) {
        let x0 = rc.top_left.x;
        let y0 = rc.top_left.y;
        let x1 = rc.right();
        let y1 = rc.bottom();
        for y in y0..=y1 {
            for x in x0..=x1 {
                r.set_pixel(x, y, color);
            }
        }
    }
}

pub fn fill_rect_rgba(r: &mut dyn Rasterizer, rect: Rect, color: Rgba8888) {
    let clip = Rect::new(Point::zero(), Size::new(r.width(), r.height()));
    if let Some(rc) = rect.intersection(&clip) {
        let rgb = color.to_rgb565();
        for y in rc.top_left.y..=rc.bottom() {
            for x in rc.top_left.x..=rc.right() {
                r.blend_pixel(x, y, rgb, color.a);
            }
        }
    }
}

pub fn fill_rect_linear_gradient(r: &mut dyn Rasterizer, rect: Rect, grad: &LinearGradient) {
    let clip = Rect::new(Point::zero(), Size::new(r.width(), r.height()));
    if let Some(rc) = rect.intersection(&clip) {
        for y in rc.top_left.y..=rc.bottom() {
            for x in rc.top_left.x..=rc.right() {
                let c = grad.sample(Point::new(x, y));
                r.set_pixel(x, y, c);
            }
        }
    }
}

pub fn fill_rect_radial_gradient(r: &mut dyn Rasterizer, rect: Rect, grad: &RadialGradient) {
    let clip = Rect::new(Point::zero(), Size::new(r.width(), r.height()));
    if let Some(rc) = rect.intersection(&clip) {
        for y in rc.top_left.y..=rc.bottom() {
            for x in rc.top_left.x..=rc.right() {
                let c = grad.sample(Point::new(x, y));
                r.set_pixel(x, y, c);
            }
        }
    }
}

// Xiaolin Wu anti-aliased line
pub fn draw_line_aa(r: &mut dyn Rasterizer, p0: Point, p1: Point, color: Rgb565) {
    fn ipart(x: f32) -> i32 { x.floor() as i32 }
    fn round(x: f32) -> i32 { (x + 0.5).floor() as i32 }
    fn fpart(x: f32) -> f32 { x - x.floor() }
    fn rfpart(x: f32) -> f32 { 1.0 - fpart(x) }

    let mut x0 = p0.x as f32;
    let mut y0 = p0.y as f32;
    let mut x1 = p1.x as f32;
    let mut y1 = p1.y as f32;
    let steep = (y1 - y0).abs() > (x1 - x0).abs();
    if steep { core::mem::swap(&mut x0, &mut y0); core::mem::swap(&mut x1, &mut y1); }
    if x0 > x1 { core::mem::swap(&mut x0, &mut x1); core::mem::swap(&mut y0, &mut y1); }
    let dx = x1 - x0;
    let dy = y1 - y0;
    let gradient = if dx == 0.0 { 1.0 } else { dy / dx };

    // first endpoint
    let xend = round(x0) as f32;
    let yend = y0 + gradient * (xend - x0);
    let xgap = rfpart(x0 + 0.5);
    let xpxl1 = xend as i32;
    let ypxl1 = ipart(yend);
    plot(r, steep, xpxl1, ypxl1, color, (rfpart(yend) * xgap));
    plot(r, steep, xpxl1, ypxl1 + 1, color, (fpart(yend) * xgap));
    let mut intery = yend + gradient;

    // second endpoint
    let xend2 = round(x1) as f32;
    let yend2 = y1 + gradient * (xend2 - x1);
    let xgap2 = fpart(x1 + 0.5);
    let xpxl2 = xend2 as i32;
    let ypxl2 = ipart(yend2);
    // main loop
    for x in (xpxl1 + 1)..(xpxl2) {
        plot(r, steep, x, ipart(intery), color, rfpart(intery));
        plot(r, steep, x, ipart(intery) + 1, color, fpart(intery));
        intery += gradient;
    }
    plot(r, steep, xpxl2, ypxl2, color, rfpart(yend2) * xgap2);
    plot(r, steep, xpxl2, ypxl2 + 1, color, fpart(yend2) * xgap2);

    fn plot(r: &mut dyn Rasterizer, steep: bool, x: i32, y: i32, color: Rgb565, a: f32) {
        let a_u8 = (a.clamp(0.0, 1.0) * 255.0) as u8;
        if steep { r.blend_pixel(y, x, color, a_u8) } else { r.blend_pixel(x, y, color, a_u8) }
    }
}

pub fn draw_line_rgba_aa(r: &mut dyn Rasterizer, p0: Point, p1: Point, color: Rgba8888) {
    // Use AA and scale per-sample alpha by color.a
    fn ipart(x: f32) -> i32 { x.floor() as i32 }
    fn round(x: f32) -> i32 { (x + 0.5).floor() as i32 }
    fn fpart(x: f32) -> f32 { x - x.floor() }
    fn rfpart(x: f32) -> f32 { 1.0 - fpart(x) }

    let mut x0 = p0.x as f32;
    let mut y0 = p0.y as f32;
    let mut x1 = p1.x as f32;
    let mut y1 = p1.y as f32;
    let steep = (y1 - y0).abs() > (x1 - x0).abs();
    if steep { core::mem::swap(&mut x0, &mut y0); core::mem::swap(&mut x1, &mut y1); }
    if x0 > x1 { core::mem::swap(&mut x0, &mut x1); core::mem::swap(&mut y0, &mut y1); }
    let dx = x1 - x0;
    let dy = y1 - y0;
    let gradient = if dx == 0.0 { 1.0 } else { dy / dx };

    let base = color.to_rgb565();
    let a_base = color.a as f32 / 255.0;

    let xend = round(x0) as f32;
    let yend = y0 + gradient * (xend - x0);
    let xgap = rfpart(x0 + 0.5);
    let xpxl1 = xend as i32;
    let ypxl1 = ipart(yend);
    plot(r, steep, xpxl1, ypxl1, base, (rfpart(yend) * xgap * a_base));
    plot(r, steep, xpxl1, ypxl1 + 1, base, (fpart(yend) * xgap * a_base));
    let mut intery = yend + gradient;

    let xend2 = round(x1) as f32;
    let yend2 = y1 + gradient * (xend2 - x1);
    let xgap2 = fpart(x1 + 0.5);
    let xpxl2 = xend2 as i32;
    let ypxl2 = ipart(yend2);
    for x in (xpxl1 + 1)..(xpxl2) {
        plot(r, steep, x, ipart(intery), base, rfpart(intery) * a_base);
        plot(r, steep, x, ipart(intery) + 1, base, fpart(intery) * a_base);
        intery += gradient;
    }
    plot(r, steep, xpxl2, ypxl2, base, rfpart(yend2) * xgap2 * a_base);
    plot(r, steep, xpxl2, ypxl2 + 1, base, fpart(yend2) * xgap2 * a_base);

    fn plot(r: &mut dyn Rasterizer, steep: bool, x: i32, y: i32, color: Rgb565, a: f32) {
        let a_u8 = (a.clamp(0.0, 1.0) * 255.0) as u8;
        if steep { r.blend_pixel(y, x, color, a_u8) } else { r.blend_pixel(x, y, color, a_u8) }
    }
}

pub fn draw_circle_aa(r: &mut dyn Rasterizer, center: Point, radius: i32, color: Rgb565) {
    // Midpoint circle with simple edge AA by coverage approximation
    let mut x = radius;
    let mut y = 0;
    let mut err = 1 - x;
    while x >= y {
        circle_points_aa(r, center, x, y, color);
        y += 1;
        if err < 0 { err += 2 * y + 1; } else { x -= 1; err += 2 * (y - x) + 1; }
    }
}

pub fn fill_circle(r: &mut dyn Rasterizer, center: Point, radius: i32, color: Rgb565) {
    let r2 = radius * radius;
    for dy in -radius..=radius {
        for dx in -radius..=radius {
            if dx * dx + dy * dy <= r2 {
                r.set_pixel(center.x + dx, center.y + dy, color);
            }
        }
    }
}

fn circle_points_aa(r: &mut dyn Rasterizer, c: Point, x: i32, y: i32, color: Rgb565) {
    let pts = [
        (c.x + x, c.y + y), (c.x - x, c.y + y), (c.x + x, c.y - y), (c.x - x, c.y - y),
        (c.x + y, c.y + x), (c.x - y, c.y + x), (c.x + y, c.y - x), (c.x - y, c.y - x),
    ];
    for &(px, py) in &pts {
        r.blend_pixel(px, py, color, 255);
    }
}

pub fn draw_rect_outline_aa(r: &mut dyn Rasterizer, rect: Rect, thickness: i32, color: Rgb565) {
    let t = thickness.max(1);
    // top
    for dy in 0..t { draw_line_aa(r, Point::new(rect.top_left.x, rect.top_left.y + dy), Point::new(rect.right(), rect.top_left.y + dy), color); }
    // bottom
    for dy in 0..t { draw_line_aa(r, Point::new(rect.top_left.x, rect.bottom() - dy), Point::new(rect.right(), rect.bottom() - dy), color); }
    // left
    for dx in 0..t { draw_line_aa(r, Point::new(rect.top_left.x + dx, rect.top_left.y), Point::new(rect.top_left.x + dx, rect.bottom()), color); }
    // right
    for dx in 0..t { draw_line_aa(r, Point::new(rect.right() - dx, rect.top_left.y), Point::new(rect.right() - dx, rect.bottom()), color); }
}

pub fn fill_rounded_rect(r: &mut dyn Rasterizer, rect: Rect, radius: i32, color: Rgb565) {
    let rx = radius.max(0).min(rect.size.width as i32 / 2).min(rect.size.height as i32 / 2);
    // center fill
    let inner = Rect::new(Point::new(rect.top_left.x + rx, rect.top_left.y), Size::new((rect.size.width as i32 - 2 * rx) as u32, rect.size.height));
    fill_rect(r, inner, color);
    // side rectangles
    let side_h = (rect.size.height as i32 - 2 * rx).max(0) as u32;
    if side_h > 0 {
        fill_rect(r, Rect::new(Point::new(rect.top_left.x, rect.top_left.y + rx), Size::new(rx as u32, side_h)), color);
        fill_rect(r, Rect::new(Point::new(rect.right() - rx + 1, rect.top_left.y + rx), Size::new(rx as u32, side_h)), color);
    }
    // corners: approximate with circle quadrants
    fill_quarter_circle(r, Point::new(rect.top_left.x + rx, rect.top_left.y + rx), rx, color, 0);
    fill_quarter_circle(r, Point::new(rect.right() - rx + 1, rect.top_left.y + rx), rx, color, 1);
    fill_quarter_circle(r, Point::new(rect.top_left.x + rx, rect.bottom() - rx + 1), rx, color, 2);
    fill_quarter_circle(r, Point::new(rect.right() - rx + 1, rect.bottom() - rx + 1), rx, color, 3);
}

fn fill_quarter_circle(r: &mut dyn Rasterizer, center: Point, radius: i32, color: Rgb565, quadrant: u8) {
    // Simple filled circle quadrant rasterization
    let r2 = radius * radius;
    for dy in 0..=radius {
        for dx in 0..=radius {
            if dx * dx + dy * dy <= r2 {
                match quadrant {
                    0 => r.set_pixel(center.x - dx, center.y - dy, color),
                    1 => r.set_pixel(center.x + dx, center.y - dy, color),
                    2 => r.set_pixel(center.x - dx, center.y + dy, color),
                    _ => r.set_pixel(center.x + dx, center.y + dy, color),
                }
            }
        }
    }
}

pub fn draw_arc_aa(r: &mut dyn Rasterizer, center: Point, radius: i32, start_angle_rad: f32, end_angle_rad: f32, color: Rgb565) {
    let steps = (radius as f32 * (end_angle_rad - start_angle_rad).abs()).max(16.0) as i32;
    let mut prev = None;
    for i in 0..=steps {
        let t = i as f32 / steps as f32;
        let ang = start_angle_rad + (end_angle_rad - start_angle_rad) * t;
        let x = center.x + (radius as f32 * ang.cos()) as i32;
        let y = center.y + (radius as f32 * ang.sin()) as i32;
        if let Some(p) = prev { draw_line_aa(r, p, Point::new(x, y), color); }
        prev = Some(Point::new(x, y));
    }
}

// ========================= Helpers =========================

pub fn clip_rect_to_surface(rect: Rect, width: u32, height: u32) -> Option<Rect> {
    let surface = Rect::new(Point::zero(), Size::new(width, height));
    rect.intersection(&surface)
}


