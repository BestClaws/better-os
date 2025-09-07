#![no_std]

use crate::libs::gfx::two_d::gradients::{LinearGradient, RadialGradient};
use micromath::F32Ext;
use crate::libs::gfx::two_d::raster::Rasterizer;
use crate::libs::gfx::two_d::types::{Point, Rect, Rgb565, Rgba8888};
use crate::libs::gfx::two_d::primitives as prim;
use crate::libs::gfx::two_d::Size;
use crate::libs::gfx::two_d::paint::{PixelSampler, Brush};

/// Fluent drawing entry-point that provides builder-pattern APIs for 2D primitives.
///
/// Design goals:
/// - Fluent, chainable configuration for stroke/fill/corner radii
/// - Robust defaulting (zero config draws sensible results)
/// - No legacy `draw_*` function exposure to call sites
/// - Space-grade performance by delegating to optimized primitives
pub struct Draw<'a> {
    pub raster: &'a mut dyn Rasterizer,
}

impl<'a> Draw<'a> {
    /// Create a new fluent drawing context wrapping a `Rasterizer`.
    #[inline(always)]
    pub fn new(raster: &'a mut dyn Rasterizer) -> Self { Self { raster } }

    /// Start a rectangle builder.
    #[inline(always)]
    pub fn rect(&'a mut self, rect: Rect) -> RectBuilder<'a> {
        RectBuilder::new(self.raster, rect)
    }

    /// Start a single line builder.
    #[inline(always)]
    pub fn line(&'a mut self, from: Point, to: Point) -> LineBuilder<'a> {
        LineBuilder::new(self.raster, from, to)
    }

    /// Start a path builder for polylines and Bezier segments.
    #[inline(always)]
    pub fn path(&'a mut self) -> PathBuilder<'a> {
        PathBuilder::new(self.raster)
    }
}

// ===== Generic paint sampling to avoid permutation explosion =====


#[inline(always)]
fn clip_rect_to_target(rect: Rect, target_size: Size) -> Option<Rect> {
    let clip = Rect::new(Point::zero(), target_size);
    rect.intersection(&clip)
}

#[inline(always)]
fn fill_rect_with<P: PixelSampler>(r: &mut dyn Rasterizer, rect: Rect, paint: &P) {
    let Some(rc) = clip_rect_to_target(rect, Size::new(r.width(), r.height())) else { return; };
    for y in rc.top_left.y..=rc.bottom() {
        for x in rc.top_left.x..=rc.right() {
            let (c, a) = paint.sample(x, y);
            if a == 255 { r.set_pixel(x, y, c); }
            else if a != 0 { r.blend_pixel(x, y, c, a); }
        }
    }
}

#[inline(always)]
fn fill_rect_masked_with<P: PixelSampler>(r: &mut dyn Rasterizer, rect: Rect, radii: CornerRadii, paint: &P) {
    let Some(rc) = clip_rect_to_target(rect, Size::new(r.width(), r.height())) else { return; };
    for y in rc.top_left.y..=rc.bottom() {
        for x in rc.top_left.x..=rc.right() {
            if inside_rounded_rect_nonuniform(x, y, rect, radii) {
                let (c, a) = paint.sample(x, y);
                if a == 255 { r.set_pixel(x, y, c); }
                else if a != 0 { r.blend_pixel(x, y, c, a); }
            }
        }
    }
}

/// Stroke style for outlines and paths.
#[derive(Clone, Copy, Debug)]
pub struct StrokeStyle {
    pub color: Rgb565,
    pub thickness: i32,
    pub aa: bool,
}

impl StrokeStyle {
    pub const fn new(color: Rgb565, thickness: i32) -> Self { Self { color, thickness, aa: true } }
    pub const fn with_aa(mut self, aa: bool) -> Self { self.aa = aa; self }
}

/// Fill style for solid and RGBA fills.
#[derive(Clone, Copy, Debug)]
pub enum FillStyle { Deprecated }

/// Per-corner radii for rounded rectangles.
#[derive(Clone, Copy, Debug, Default)]
pub struct CornerRadii { pub tl: i32, pub tr: i32, pub br: i32, pub bl: i32 }

impl CornerRadii {
    pub const fn uniform(r: i32) -> Self { Self { tl: r, tr: r, br: r, bl: r } }
    pub fn is_uniform(&self) -> bool { self.tl == self.tr && self.tr == self.br && self.br == self.bl }
    pub fn max_uniform(&self) -> i32 { if self.is_uniform() { self.tl } else { self.tl.min(self.tr).min(self.br).min(self.bl) } }
}

/// Optional outer shadow spec for rounded rectangles.
#[derive(Clone, Copy, Debug)]
pub struct OuterShadow { pub blur_radius: i32, pub color: Rgb565, pub max_alpha: u8 }

/// Rectangle builder supporting stroke, fill, radii and gradient.
pub struct RectBuilder<'a> {
    raster: &'a mut dyn Rasterizer,
    rect: Rect,
    stroke: Option<StrokeStyle>,
    fill: Option<Brush>,
    radii: CornerRadii,
    outer_shadow: Option<OuterShadow>,
}

impl<'a> RectBuilder<'a> {
    #[inline(always)]
    fn new(raster: &'a mut dyn Rasterizer, rect: Rect) -> Self {
        Self { raster, rect, stroke: None, fill: None, radii: CornerRadii::default(), outer_shadow: None }
    }

    /// Set a uniform corner radius.
    pub fn corner_radius(mut self, r: i32) -> Self { self.radii = CornerRadii::uniform(r); self }

    /// Set per-corner radii.
    pub fn corner_radii(mut self, radii: CornerRadii) -> Self { self.radii = radii; self }

    /// Apply a solid fill color.
    pub fn fill_color(mut self, color: Rgb565) -> Self { self.fill = Some(Brush::solid(color)); self }

    /// Apply an RGBA fill color (alpha-blended).
    pub fn fill_rgba(mut self, color: Rgba8888) -> Self { self.fill = Some(Brush::rgba(color)); self }

    /// Apply a fill kind (solid, RGBA, gradient, etc.).
    pub fn fill(mut self, brush: Brush) -> Self { self.fill = Some(brush); self }

    /// Apply stroke style.
    pub fn stroke(mut self, style: StrokeStyle) -> Self { self.stroke = Some(style); self }

    /// Convenience: set stroke color and thickness.
    pub fn stroke_color(mut self, color: Rgb565) -> Self { self.stroke = Some(StrokeStyle { color, thickness: 1, aa: true }); self }

    /// Convenience: set stroke thickness.
    pub fn stroke_width(mut self, thickness: i32) -> Self {
        if let Some(mut s) = self.stroke { s.thickness = thickness; self.stroke = Some(s); } else { self.stroke = Some(StrokeStyle { color: Rgb565::WHITE, thickness, aa: true }); }
        self
    }

    /// Add an outer rounded-rect shadow drawn before fill and stroke.
    pub fn outer_shadow(mut self, blur_radius: i32, color: Rgb565, max_alpha: u8) -> Self {
        self.outer_shadow = Some(OuterShadow { blur_radius, color, max_alpha });
        self
    }

    /// Convenience: rounded corners with default radius (6px).
    pub fn rounded(mut self) -> Self { self.radii = CornerRadii::uniform(6); self }

    /// Rasterize with configured options.
    pub fn draw(self) {
        // Optional outer shadow first
        if let Some(sh) = self.outer_shadow {
            // For non-uniform radii, approximate with min radius (visually acceptable for soft shadows)
            prim::draw_rounded_rect_shadow_layers(self.raster, self.rect, self.radii.max_uniform(), sh.blur_radius, sh.color, sh.max_alpha);
        }

        // Fill
        if let Some(fill) = self.fill {
            match fill {
                Brush::Solid(c) => {
                    if self.radii.is_uniform() {
                        let r = self.radii.max_uniform();
                        if r > 0 { prim::fill_rounded_rect(self.raster, self.rect, r, c); }
                        else { prim::fill_rect(self.raster, self.rect, c); }
                    } else {
                        fill_rect_nonuniform_solid(self.raster, self.rect, self.radii, c);
                    }
                }
                Brush::Rgba(c) => {
                    let r = self.radii.max_uniform();
                    if self.radii.is_uniform() && r > 0 {
                        // Keep optimized rounded RGBA for uniform radii
                        prim::fill_rounded_rect_rgba(self.raster, self.rect, r, c);
                    } else if self.radii.is_uniform() && r == 0 {
                        fill_rect_with(self.raster, self.rect, &c);
                    } else {
                        fill_rect_masked_with(self.raster, self.rect, self.radii, &c);
                    }
                }
                Brush::Linear(grad) => {
                    // Use rect-resolved gradient spec
                    // Expect a concrete LinearGradient was provided via Brush::Linear
                    let grad = if let Some(Brush::Linear(g)) = self.fill { g } else { LinearGradient::new(self.rect.top_left, Point::new(self.rect.right(), self.rect.top_left.y), Rgb565::BLACK, Rgb565::WHITE) };
                    let r = self.radii.max_uniform();
                    if self.radii.is_uniform() && r == 0 {
                        fill_rect_with(self.raster, self.rect, &grad);
                    } else {
                        fill_rect_masked_with(self.raster, self.rect, self.radii, &grad);
                    }
                }
                Brush::Radial(grad) => {
                    let r = self.radii.max_uniform();
                    if self.radii.is_uniform() && r == 0 { fill_rect_with(self.raster, self.rect, &grad); }
                    else { fill_rect_masked_with(self.raster, self.rect, self.radii, &grad); }
                }
            }
        }

        // Stroke
        if let Some(st) = self.stroke {
            if st.thickness > 0 {
                if self.radii.is_uniform() {
                    let r = self.radii.max_uniform();
                    if r > 0 {
                        prim::draw_rounded_rect_outline_aa(self.raster, self.rect, r, st.thickness, st.color);
                    } else {
                        prim::draw_rect_outline_aa(self.raster, self.rect, st.thickness, st.color);
                    }
                } else {
                    stroke_rect_nonuniform(self.raster, self.rect, self.radii, st);
                }
            }
        }
    }
}

/// Single segment line builder.
pub struct LineBuilder<'a> {
    raster: &'a mut dyn Rasterizer,
    from: Point,
    to: Point,
    style: StrokeStyle,
    rgba: Option<Rgba8888>,
}

impl<'a> LineBuilder<'a> {
    #[inline(always)]
    fn new(raster: &'a mut dyn Rasterizer, from: Point, to: Point) -> Self {
        Self { raster, from, to, style: StrokeStyle { color: Rgb565::WHITE, thickness: 1, aa: true }, rgba: None }
    }
    pub fn color(mut self, color: Rgb565) -> Self { self.style.color = color; self }
    pub fn color_rgba(mut self, color: Rgba8888) -> Self { self.rgba = Some(color); self }
    pub fn thickness(mut self, thickness: i32) -> Self { self.style.thickness = thickness; self }
    pub fn aa(mut self, aa: bool) -> Self { self.style.aa = aa; self }
    pub fn draw(self) {
        // Unified line rendering path
        if let Some(rgba) = self.rgba {
            if self.style.thickness <= 1 {
                prim::draw_line_rgba_aa(self.raster, self.from, self.to, rgba);
            } else {
                // Approx thick RGBA by multiple AA passes
                let dx = (self.to.x - self.from.x) as f32;
                let dy = (self.to.y - self.from.y) as f32;
                let len = (dx * dx + dy * dy).sqrt();
                if len == 0.0 {
                    let r = (self.style.thickness / 2).max(1);
                    prim::fill_circle(self.raster, self.from, r, rgba.to_rgb565());
                } else {
                    let nx = -dy / len; let ny = dx / len;
                    let half = (self.style.thickness as f32) / 2.0;
                    let steps = self.style.thickness.max(1);
                    for i in 0..steps {
                        let t = (i as f32 + 0.5) - half;
                        let off_x = (nx * t).round() as i32;
                        let off_y = (ny * t).round() as i32;
                        prim::draw_line_rgba_aa(
                            self.raster,
                            Point::new(self.from.x + off_x, self.from.y + off_y),
                            Point::new(self.to.x + off_x, self.to.y + off_y),
                            rgba,
                        );
                    }
                }
            }
        } else if self.style.thickness <= 1 {
            if self.style.aa { prim::draw_line_aa(self.raster, self.from, self.to, self.style.color); }
            else { prim::draw_arc(self.raster, self.from, 0, 0.0, 0.0, self.style.color); }
        } else {
            prim::draw_line_thick_aa(self.raster, self.from, self.to, self.style.thickness, self.style.color);
        }
    }
}

/// Path builder supporting `move_to`, `line_to`, and Bezier segments (approximated).
pub struct PathBuilder<'a> {
    raster: &'a mut dyn Rasterizer,
    pen: Option<Point>,
    start: Option<Point>,
    style: StrokeStyle,
}

impl<'a> PathBuilder<'a> {
    #[inline(always)]
    fn new(raster: &'a mut dyn Rasterizer) -> Self {
        Self { raster, pen: None, start: None, style: StrokeStyle { color: Rgb565::WHITE, thickness: 1, aa: true } }
    }
    pub fn stroke(mut self, style: StrokeStyle) -> Self { self.style = style; self }
    pub fn color(mut self, color: Rgb565) -> Self { self.style.color = color; self }
    pub fn thickness(mut self, thickness: i32) -> Self { self.style.thickness = thickness; self }
    pub fn aa(mut self, aa: bool) -> Self { self.style.aa = aa; self }

    /// Set the current position without drawing.
    pub fn move_to(mut self, p: Point) -> Self { self.pen = Some(p); self.start = Some(p); self }

    /// Draw a line to `p` from the current pen position.
    pub fn line_to(mut self, p: Point) -> Self {
        if let Some(prev) = self.pen {
            if self.style.thickness <= 1 { prim::draw_line_aa(self.raster, prev, p, self.style.color); }
            else { prim::draw_line_thick_aa(self.raster, prev, p, self.style.thickness, self.style.color); }
            self.pen = Some(p);
        } else {
            self.pen = Some(p);
            if self.start.is_none() { self.start = Some(p); }
        }
        self
    }

    /// Quadratic Bezier to point `p` with control `c` (approximated with segments).
    pub fn quadratic_to(mut self, c: Point, p: Point) -> Self {
        if let Some(prev) = self.pen { approximate_quadratic(self.raster, prev, c, p, self.style); self.pen = Some(p); }
        self
    }

    /// Cubic Bezier to point `p` with controls `c1`, `c2` (approximated with segments).
    pub fn cubic_to(mut self, c1: Point, c2: Point, p: Point) -> Self {
        if let Some(prev) = self.pen { approximate_cubic(self.raster, prev, c1, c2, p, self.style); self.pen = Some(p); }
        self
    }

    /// Close path by drawing a segment to the start point if present.
    pub fn close(mut self) -> Self {
        if let (Some(start), Some(cur)) = (self.start, self.pen) {
            if cur != start {
                if self.style.thickness <= 1 { prim::draw_line_aa(self.raster, cur, start, self.style.color); }
                else { prim::draw_line_thick_aa(self.raster, cur, start, self.style.thickness, self.style.color); }
            }
        }
        self
    }

    /// Finish path (no-op for now).
    pub fn finish(self) { /* builder consumed */ }
}

// ===== Bezier approximations =====

#[inline(always)]
fn lerp(a: f32, b: f32, t: f32) -> f32 { a + (b - a) * t }

fn approximate_quadratic(r: &mut dyn Rasterizer, p0: Point, c: Point, p1: Point, style: StrokeStyle) {
    // Adaptive step count based on curve length (clamped)
    let dx = (p1.x - p0.x) as f32; let dy = (p1.y - p0.y) as f32;
    let len = (dx * dx + dy * dy).sqrt();
    let steps = (len / 6.0).clamp(8.0, 64.0) as i32;
    let mut prev = p0;
    for i in 1..=steps {
        let t = i as f32 / steps as f32;
        let it = 1.0 - t;
        let x = it * it * p0.x as f32 + 2.0 * it * t * c.x as f32 + t * t * p1.x as f32;
        let y = it * it * p0.y as f32 + 2.0 * it * t * c.y as f32 + t * t * p1.y as f32;
        let pt = Point::new(x.round() as i32, y.round() as i32);
        if style.thickness <= 1 { prim::draw_line_aa(r, prev, pt, style.color); }
        else { prim::draw_line_thick_aa(r, prev, pt, style.thickness, style.color); }
        prev = pt;
    }
}

fn approximate_cubic(r: &mut dyn Rasterizer, p0: Point, c1: Point, c2: Point, p1: Point, style: StrokeStyle) {
    let dx = (p1.x - p0.x) as f32; let dy = (p1.y - p0.y) as f32;
    let len = (dx * dx + dy * dy).sqrt();
    let steps = (len / 5.0).clamp(12.0, 96.0) as i32;
    let mut prev = p0;
    for i in 1..=steps {
        let t = i as f32 / steps as f32;
        let it = 1.0 - t;
        // De Casteljau
        let x =
            it * it * it * p0.x as f32 +
            3.0 * it * it * t * c1.x as f32 +
            3.0 * it * t * t * c2.x as f32 +
            t * t * t * p1.x as f32;
        let y =
            it * it * it * p0.y as f32 +
            3.0 * it * it * t * c1.y as f32 +
            3.0 * it * t * t * c2.y as f32 +
            t * t * t * p1.y as f32;
        let pt = Point::new(x.round() as i32, y.round() as i32);
        if style.thickness <= 1 { prim::draw_line_aa(r, prev, pt, style.color); }
        else { prim::draw_line_thick_aa(r, prev, pt, style.thickness, style.color); }
        prev = pt;
    }
}

// ===== Public helpers for ergonomic API =====

#[derive(Clone, Copy, Debug)]
pub enum LinearMode { Horizontal, Vertical, Angle(f32) }

#[derive(Clone, Copy, Debug)]
pub struct GradientSpec { pub mode: LinearMode, pub a: Rgb565, pub b: Rgb565 }

#[inline(always)]
pub fn gradient(a: Rgb565, b: Rgb565) -> GradientSpec { GradientSpec { mode: LinearMode::Horizontal, a, b } }

#[inline(always)]
pub fn gradient_vertical(a: Rgb565, b: Rgb565) -> GradientSpec { GradientSpec { mode: LinearMode::Vertical, a, b } }

#[inline(always)]
pub fn gradient_angle(deg: f32, a: Rgb565, b: Rgb565) -> GradientSpec { GradientSpec { mode: LinearMode::Angle(deg), a, b } }

#[inline(always)]
pub fn stroke(thickness: i32, color: Rgb565) -> StrokeStyle { StrokeStyle { color, thickness, aa: true } }

#[derive(Clone, Copy, Debug)]
pub struct RadialSpec { pub center: Point, pub radius: u32, pub inner: Rgb565, pub outer: Rgb565 }

#[inline(always)]
pub fn radial(center: Point, radius: u32, inner: Rgb565, outer: Rgb565) -> RadialSpec { RadialSpec { center, radius, inner, outer } }

// ===== Arc builder =====

pub struct ArcBuilder<'a> {
    raster: &'a mut dyn Rasterizer,
    center: Point,
    radius: i32,
    start: f32,
    end: f32,
    color: Rgb565,
}

impl<'a> ArcBuilder<'a> {
    #[inline(always)]
    fn new(raster: &'a mut dyn Rasterizer, center: Point, radius: i32, start: f32, end: f32) -> Self {
        Self { raster, center, radius, start, end, color: Rgb565::WHITE }
    }
    pub fn color(mut self, color: Rgb565) -> Self { self.color = color; self }
    pub fn draw(self) { prim::draw_arc_aa(self.raster, self.center, self.radius, self.start, self.end, self.color); }
}

impl<'a> Draw<'a> {
    #[inline(always)]
    pub fn arc(&'a mut self, center: Point, radius: i32, start: f32, end: f32) -> ArcBuilder<'a> {
        ArcBuilder::new(self.raster, center, radius, start, end)
    }
}

// ===== Non-uniform rounded rectangle helpers =====

#[inline(always)]
fn inside_rounded_rect_nonuniform(x: i32, y: i32, rect: Rect, radii: CornerRadii) -> bool {
    let left = rect.top_left.x;
    let right = rect.right();
    let top = rect.top_left.y;
    let bottom = rect.bottom();

    // Core band extents using max radii per side
    let core_left = radii.tl.max(radii.bl);
    let core_right = radii.tr.max(radii.br);
    let core_top = radii.tl.max(radii.tr);
    let core_bottom = radii.bl.max(radii.br);

    // Center vertical band
    if x >= left + core_left && x <= right - core_right { return y >= top && y <= bottom; }
    // Center horizontal band
    if y >= top + core_top && y <= bottom - core_bottom { return x >= left && x <= right; }

    // Side rectangles between corner arcs
    // Left side
    if x >= left && x < left + core_left {
        if y >= top + radii.tl && y <= bottom - radii.bl { return true; }
    }
    // Right side
    if x > right - core_right && x <= right {
        if y >= top + radii.tr && y <= bottom - radii.br { return true; }
    }
    // Top side
    if y >= top && y < top + core_top {
        if x >= left + radii.tl && x <= right - radii.tr { return true; }
    }
    // Bottom side
    if y > bottom - core_bottom && y <= bottom {
        if x >= left + radii.bl && x <= right - radii.br { return true; }
    }

    // Corner circular checks
    // TL
    if x < left + radii.tl && y < top + radii.tl {
        let cx = left + radii.tl; let cy = top + radii.tl;
        let dx = x - cx; let dy = y - cy; return dx*dx + dy*dy <= radii.tl*radii.tl;
    }
    // TR
    if x > right - radii.tr && y < top + radii.tr {
        let cx = right - radii.tr; let cy = top + radii.tr;
        let dx = x - cx; let dy = y - cy; return dx*dx + dy*dy <= radii.tr*radii.tr;
    }
    // BL
    if x < left + radii.bl && y > bottom - radii.bl {
        let cx = left + radii.bl; let cy = bottom - radii.bl;
        let dx = x - cx; let dy = y - cy; return dx*dx + dy*dy <= radii.bl*radii.bl;
    }
    // BR
    if x > right - radii.br && y > bottom - radii.br {
        let cx = right - radii.br; let cy = bottom - radii.br;
        let dx = x - cx; let dy = y - cy; return dx*dx + dy*dy <= radii.br*radii.br;
    }
    false
}

fn fill_rect_nonuniform_solid(r: &mut dyn Rasterizer, rect: Rect, radii: CornerRadii, color: Rgb565) {
    for y in rect.top_left.y..=rect.bottom() {
        for x in rect.top_left.x..=rect.right() {
            if inside_rounded_rect_nonuniform(x, y, rect, radii) { r.set_pixel(x, y, color); }
        }
    }
}

fn fill_rect_nonuniform_rgba(r: &mut dyn Rasterizer, rect: Rect, radii: CornerRadii, color: Rgba8888) {
    let rgb = color.to_rgb565();
    let alpha = color.a;
    if alpha == 0 { return; }
    if alpha == 255 {
        fill_rect_nonuniform_solid(r, rect, radii, rgb);
        return;
    }
    for y in rect.top_left.y..=rect.bottom() {
        for x in rect.top_left.x..=rect.right() {
            if inside_rounded_rect_nonuniform(x, y, rect, radii) { r.blend_pixel(x, y, rgb, alpha); }
        }
    }
}

fn fill_rect_nonuniform_gradient(r: &mut dyn Rasterizer, rect: Rect, radii: CornerRadii, grad: &LinearGradient) {
    for y in rect.top_left.y..=rect.bottom() {
        for x in rect.top_left.x..=rect.right() {
            if inside_rounded_rect_nonuniform(x, y, rect, radii) {
                let c = grad.sample(Point::new(x, y));
                r.set_pixel(x, y, c);
            }
        }
    }
}

fn stroke_rect_nonuniform(r: &mut dyn Rasterizer, rect: Rect, radii: CornerRadii, style: StrokeStyle) {
    let left = rect.top_left.x;
    let right = rect.right();
    let top = rect.top_left.y;
    let bottom = rect.bottom();
    let t = style.thickness.max(1);
    // Edges between corners: repeat for thickness layering
    for d in 0..t {
        // Top
        let y_top = top + d;
        if left + radii.tl <= right - radii.tr {
            prim::draw_line_aa(r, Point::new(left + radii.tl, y_top), Point::new(right - radii.tr, y_top), style.color);
        }
        // Bottom
        let y_bottom = bottom - d;
        if left + radii.bl <= right - radii.br {
            prim::draw_line_aa(r, Point::new(left + radii.bl, y_bottom), Point::new(right - radii.br, y_bottom), style.color);
        }
        // Left
        let x_left = left + d;
        if top + radii.tl <= bottom - radii.bl {
            prim::draw_line_aa(r, Point::new(x_left, top + radii.tl), Point::new(x_left, bottom - radii.bl), style.color);
        }
        // Right
        let x_right = right - d;
        if top + radii.tr <= bottom - radii.br {
            prim::draw_line_aa(r, Point::new(x_right, top + radii.tr), Point::new(x_right, bottom - radii.br), style.color);
        }
    }
    // Corner arcs per thickness layer
    for d in 0..t {
        let rtl = (radii.tl - d).max(0);
        let rtr = (radii.tr - d).max(0);
        let rbl = (radii.bl - d).max(0);
        let rbr = (radii.br - d).max(0);
        if rtl > 0 { prim::draw_arc_aa(r, Point::new(left + radii.tl, top + radii.tl), rtl, core::f32::consts::PI, 1.5 * core::f32::consts::PI, style.color); }
        if rtr > 0 { prim::draw_arc_aa(r, Point::new(right - radii.tr, top + radii.tr), rtr, 1.5 * core::f32::consts::PI, 2.0 * core::f32::consts::PI, style.color); }
        if rbl > 0 { prim::draw_arc_aa(r, Point::new(left + radii.bl, bottom - radii.bl), rbl, 0.5 * core::f32::consts::PI, core::f32::consts::PI, style.color); }
        if rbr > 0 { prim::draw_arc_aa(r, Point::new(right - radii.br, bottom - radii.br), rbr, 0.0, 0.5 * core::f32::consts::PI, style.color); }
    }
}

fn fill_rect_nonuniform_radial(r: &mut dyn Rasterizer, rect: Rect, radii: CornerRadii, grad: &RadialGradient) {
    for y in rect.top_left.y..=rect.bottom() {
        for x in rect.top_left.x..=rect.right() {
            if inside_rounded_rect_nonuniform(x, y, rect, radii) {
                let c = grad.sample(Point::new(x, y));
                r.set_pixel(x, y, c);
            }
        }
    }
}


