use crate::color::Rgba8888;
use crate::primitives::line::{draw_line, LineDsc};
use crate::primitives::triangle::{draw_triangle, TriangleDsc};
use crate::types::{Gradient, Point, OPA_70, OPA_COVER};
use crate::Rasterizer;
use micromath::F32Ext;

/// Placeholder vector rendering until full ThorVG integration lands
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum VectorStubKind {
    StarGradient,
    WaveStroke,
}

/// Minimal descriptor mirroring LVGL's draw API shape selection
#[derive(Clone, Debug, PartialEq)]
pub struct VectorDsc {
    pub kind: VectorStubKind,
}

impl VectorDsc {
    pub fn star_gradient() -> Self {
        Self {
            kind: VectorStubKind::StarGradient,
        }
    }

    pub fn wave_stroke() -> Self {
        Self {
            kind: VectorStubKind::WaveStroke,
        }
    }
}

/// Draw vector placeholder while real vector backend is pending
pub fn draw_vector<R: Rasterizer>(rast: &mut R, dsc: &VectorDsc) {
    match dsc.kind {
        VectorStubKind::StarGradient => draw_star_placeholder(rast),
        VectorStubKind::WaveStroke => draw_wave_placeholder(rast),
    }
}

fn draw_star_placeholder<R: Rasterizer>(rast: &mut R) {
    let star_pts = [
        Point::new(51, 25),
        Point::new(62, 45),
        Point::new(85, 48),
        Point::new(66, 62),
        Point::new(74, 86),
        Point::new(51, 72),
        Point::new(28, 86),
        Point::new(36, 62),
        Point::new(17, 48),
        Point::new(40, 45),
    ];
    let center = Point::new(51, 55);

    for i in 0..star_pts.len() {
        let next = (i + 1) % star_pts.len();
        let tri_dsc = TriangleDsc {
            p1: center,
            p2: star_pts[i],
            p3: star_pts[next],
            color: star_color(center, star_pts[i], star_pts[next]),
            opa: OPA_COVER,
            grad: Gradient::none(),
        };
        draw_triangle(rast, &tri_dsc);
    }

    let mut line_dsc = LineDsc {
        p1: Point::new(0, 0),
        p2: Point::new(0, 0),
        width: 3,
        color: Rgba8888::rgb(255, 255, 255),
        opa: OPA_70,
        dash_width: 0,
        dash_gap: 0,
        round_start: true,
        round_end: true,
    };

    for i in 0..star_pts.len() {
        let next = (i + 1) % star_pts.len();
        line_dsc.p1 = star_pts[i];
        line_dsc.p2 = star_pts[next];
        draw_line(rast, &line_dsc);
    }
}

fn star_color(center: Point, a: Point, b: Point) -> Rgba8888 {
    let cx = (center.x + a.x + b.x) as f32 / 3.0;
    let cy = (center.y + a.y + b.y) as f32 / 3.0;

    let x0 = 25.0f32;
    let y0 = 30.0f32;
    let x1 = 80.0f32;
    let y1 = 95.0f32;
    let dx = x1 - x0;
    let dy = y1 - y0;
    let len_sq = dx * dx + dy * dy;
    let mut t = if len_sq > 0.0 {
        ((cx - x0) * dx + (cy - y0) * dy) / len_sq
    } else {
        0.0
    };
    if t < 0.0 {
        t = 0.0;
    }
    if t > 1.0 {
        t = 1.0;
    }

    let mid_frac = 130.0f32 / 255.0f32;
    let (r, g, b) = if t <= mid_frac {
        let lt = if mid_frac > 0.0 { t / mid_frac } else { 0.0 };
        let r0 = 255.0;
        let g0 = 90.0;
        let b0 = 0.0;
        let r1 = 255.0;
        let g1 = 0.0;
        let b1 = 200.0;
        (
            r0 + (r1 - r0) * lt,
            g0 + (g1 - g0) * lt,
            b0 + (b1 - b0) * lt,
        )
    } else {
        let lt = if 1.0 - mid_frac > 0.0 {
            (t - mid_frac) / (1.0 - mid_frac)
        } else {
            0.0
        };
        let r1 = 255.0;
        let g1 = 0.0;
        let b1 = 200.0;
        let r2 = 80.0;
        let g2 = 200.0;
        let b2 = 255.0;
        (
            r1 + (r2 - r1) * lt,
            g1 + (g2 - g1) * lt,
            b1 + (b2 - b1) * lt,
        )
    };

    Rgba8888::rgba(r.round() as u8, g.round() as u8, b.round() as u8, 255)
}

fn draw_wave_placeholder<R: Rasterizer>(rast: &mut R) {
    let segments = 36;
    let mut line_dsc = LineDsc {
        p1: Point::new(0, 0),
        p2: Point::new(0, 0),
        width: 6,
        color: Rgba8888::rgb(120, 255, 120),
        opa: OPA_COVER,
        dash_width: 14,
        dash_gap: 6,
        round_start: true,
        round_end: true,
    };

    let (mut prev_x, mut prev_y) = wave_point(0.0);
    for i in 1..=segments {
        let t = i as f32 / segments as f32;
        let (curr_x, curr_y) = wave_point(t);
        line_dsc.p1 = Point::new(prev_x.round() as i32, prev_y.round() as i32);
        line_dsc.p2 = Point::new(curr_x.round() as i32, curr_y.round() as i32);
        line_dsc.color = wave_color(t);
        draw_line(rast, &line_dsc);
        prev_x = curr_x;
        prev_y = curr_y;
    }
}

fn wave_component(p0: f32, c1: f32, c2: f32, p1: f32, t: f32) -> f32 {
    let it = 1.0 - t;
    it * it * it * p0 + 3.0 * it * it * t * c1 + 3.0 * it * t * t * c2 + t * t * t * p1
}

fn wave_point(t: f32) -> (f32, f32) {
    if t < 0.5 {
        let lt = t * 2.0;
        let p0 = (22.0, 82.0);
        let c1 = (35.0, 35.0);
        let c2 = (65.0, 95.0);
        let p1 = (84.0, 44.0);
        (
            wave_component(p0.0, c1.0, c2.0, p1.0, lt),
            wave_component(p0.1, c1.1, c2.1, p1.1, lt),
        )
    } else {
        let lt = (t - 0.5) * 2.0;
        let p0 = (84.0, 44.0);
        let c1 = (70.0, 24.0);
        let c2 = (40.0, 24.0);
        let p1 = (26.0, 46.0);
        (
            wave_component(p0.0, c1.0, c2.0, p1.0, lt),
            wave_component(p0.1, c1.1, c2.1, p1.1, lt),
        )
    }
}

fn wave_color(t: f32) -> Rgba8888 {
    let mut clamped = t;
    if clamped < 0.0 {
        clamped = 0.0;
    }
    if clamped > 1.0 {
        clamped = 1.0;
    }
    let r = 120.0 + (0.0 - 120.0) * clamped;
    let g = 255.0 + (150.0 - 255.0) * clamped;
    let b = 120.0 + (255.0 - 120.0) * clamped;
    Rgba8888::rgba(r.round() as u8, g.round() as u8, b.round() as u8, 255)
}
