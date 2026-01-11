use crate::libs::gfx::color::Rgba8888;
use crate::libs::gfx::lerp_rgba;

/// Stroke descriptor shared by all primitives.
#[derive(Clone, Copy, Debug)]
pub struct StrokeStyle {
    width: i32,
    color: Rgba8888,
    alpha_scale: u8,
}

impl StrokeStyle {
    #[inline]
    pub const fn disabled() -> Self {
        Self {
            width: 0,
            color: Rgba8888::TRANSPARENT,
            alpha_scale: 255,
        }
    }

    #[inline]
    pub const fn new(width: i32, color: Rgba8888) -> Self {
        Self {
            width,
            color,
            alpha_scale: 255,
        }
    }

    #[inline]
    pub fn set(&mut self, width: i32, color: Rgba8888) {
        self.width = width.max(0);
        self.color = color;
    }

    #[inline]
    pub fn set_alpha(&mut self, alpha: u8) {
        self.alpha_scale = alpha;
    }

    #[inline]
    pub fn width(&self) -> i32 {
        self.width.max(0)
    }

    #[inline]
    pub fn effective_color(&self) -> Option<Rgba8888> {
        if self.width() == 0 {
            return None;
        }
        let color = self.color.multiply_alpha(self.alpha_scale);
        if color.alpha() == 0 {
            None
        } else {
            Some(color)
        }
    }
}

/// Context passed to fill shaders so callers can reuse distance calculations.
#[derive(Clone, Copy, Debug)]
pub struct FillContext {
    pub x: i32,
    pub y: i32,
    pub dist2: Option<i32>,
}

impl FillContext {
    #[inline]
    pub const fn new(x: i32, y: i32) -> Self {
        Self { x, y, dist2: None }
    }

    #[inline]
    pub const fn with_distance(x: i32, y: i32, dist2: i32) -> Self {
        Self {
            x,
            y,
            dist2: Some(dist2),
        }
    }
}

/// Unified fill styles for primitives with optional precomputed geometry.
#[derive(Clone, Copy, Debug)]
pub enum Fill {
    Solid {
        color: Rgba8888,
    },
    RadialGradient {
        inner: Rgba8888,
        outer: Rgba8888,
        center_x: i32,
        center_y: i32,
        radius_sq: i64,
    },
    LinearGradientH {
        start: Rgba8888,
        end: Rgba8888,
        origin: i32,
        length: i32,
    },
    LinearGradientV {
        start: Rgba8888,
        end: Rgba8888,
        origin: i32,
        length: i32,
    },
}

impl Fill {
    #[inline]
    pub const fn solid(color: Rgba8888) -> Self {
        Fill::Solid { color }
    }

    #[inline]
    pub fn radial(
        center_x: i32,
        center_y: i32,
        radius_sq: i64,
        inner: Rgba8888,
        outer: Rgba8888,
    ) -> Self {
        let sanitized = radius_sq.max(1);
        Fill::RadialGradient {
            inner,
            outer,
            center_x,
            center_y,
            radius_sq: sanitized,
        }
    }

    #[inline]
    pub fn linear_horizontal(start: Rgba8888, end: Rgba8888, origin: i32, length: i32) -> Self {
        let sanitized = length.max(1);
        Fill::LinearGradientH {
            start,
            end,
            origin,
            length: sanitized,
        }
    }

    #[inline]
    pub fn linear_vertical(start: Rgba8888, end: Rgba8888, origin: i32, length: i32) -> Self {
        let sanitized = length.max(1);
        Fill::LinearGradientV {
            start,
            end,
            origin,
            length: sanitized,
        }
    }

    #[inline]
    pub fn shade(&self, ctx: FillContext) -> Rgba8888 {
        match *self {
            Fill::Solid { color } => color,
            Fill::RadialGradient {
                inner,
                outer,
                center_x,
                center_y,
                radius_sq,
            } => {
                if radius_sq <= 0 {
                    return outer;
                }
                let dist2 = ctx.dist2.unwrap_or_else(|| {
                    let dx = ctx.x - center_x;
                    let dy = ctx.y - center_y;
                    dx * dx + dy * dy
                }) as i64;
                let numerator = dist2.saturating_mul(255);
                let denom = radius_sq.max(1);
                let frac = ((numerator / denom).clamp(0, 255)) as u8;
                lerp_rgba(inner, outer, frac)
            }
            Fill::LinearGradientH {
                start,
                end,
                origin,
                length,
            } => shade_linear(start, end, ctx.x, origin, length),
            Fill::LinearGradientV {
                start,
                end,
                origin,
                length,
            } => shade_linear(start, end, ctx.y, origin, length),
        }
    }
}

#[inline]
fn shade_linear(start: Rgba8888, end: Rgba8888, coord: i32, origin: i32, length: i32) -> Rgba8888 {
    if length <= 0 {
        return end;
    }
    let span = length.max(1) as i64;
    let pos = (coord - origin) as i64;
    let clamped = if pos <= 0 {
        0
    } else if pos >= span {
        span
    } else {
        pos
    };
    let frac = ((clamped * 255) / span) as u8;
    lerp_rgba(start, end, frac)
}
