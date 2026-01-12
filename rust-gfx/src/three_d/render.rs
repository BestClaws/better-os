use super::math::{Quaternion, Vec3};
use super::model::{Model, MAX_TRIANGLES, MAX_VERTICES};
use crate::color::Rgba8888;
use crate::rasterizer::Rasterizer;
use micromath::F32Ext;

/// Configuration for rendering the 3D model.
/// All features can be toggled individually for performance vs quality trade-offs.
#[derive(Clone, Copy)]
pub struct RenderOptions {
    /// Field of view (in degrees)
    pub fov_deg: f32,
    /// Directional light vector (normalized is preferred)
    pub light_dir: Vec3,
    /// Tuple of (min, max) intensity range to map lighting to grayscale
    pub intensity_range: (f32, f32),

    /// Enable back-face culling
    pub enable_backface_culling: bool,
    /// Enable Z-buffer (not implemented yet)
    pub enable_zbuffer: bool,
    /// Enable lighting
    pub enable_lighting: bool,
    /// Enable triangle sorting (Painter's Algorithm)
    pub enable_depth_sorting: bool,
    /// Enable clipping near plane
    pub enable_near_clipping: bool,
    /// Enable projection bounds clipping
    pub enable_frustum_clipping: bool,
    /// Enable wireframe overlay
    pub enable_wireframe: bool,
    /// Enable filled triangle shading
    pub enable_shading: bool,
    /// Enable anti-aliasing using pixel coverage estimation (experimental)
    pub enable_antialiasing: bool,
    /// Supersampling factor (1 = no AA, 2 = 2x2, etc.)
    pub antialiasing_factor: u8,
    /// Use edge-only antialiasing instead of full supersampling
    pub edge_only_antialiasing: bool,
}

#[inline(always)]
fn get_grayscale_color(intensity: f32) -> Rgba8888 {
    let clamped = intensity.clamp(0.0, 1.0);
    let value = (clamped * 255.0).round() as u8;
    Rgba8888::rgba(value, value, value, 255)
}

#[derive(Clone, Copy)]
struct ScreenPoint {
    x: i32,
    y: i32,
}

#[inline(always)]
fn project(
    v: Vec3,
    fov_deg: f32,
    width: u32,
    height: u32,
    clip_near: bool,
    clip_bounds: bool,
) -> Option<ScreenPoint> {
    if clip_near && v.2 <= 0.5 {
        return None;
    }

    let fov_rad = fov_deg.to_radians();
    let f = 1.0 / (fov_rad / 2.0).tan();
    let aspect = width as f32 / height as f32;

    let x_proj = (v.0 * f) / v.2;
    let y_proj = (v.1 * f) / v.2;

    let x = ((x_proj / aspect + 1.0) * width as f32 * 0.5) as i32;
    let y = ((1.0 - y_proj) * height as f32 * 0.5) as i32;

    if clip_bounds && (x < 0 || x >= width as i32 || y < 0 || y >= height as i32) {
        return None;
    }

    Some(ScreenPoint { x, y })
}

struct ScreenTriangle {
    p0: ScreenPoint,
    p1: ScreenPoint,
    p2: ScreenPoint,
}

fn apply_antialiasing(base: Rgba8888, x: i32, y: i32, tri: &ScreenTriangle) -> Rgba8888 {
    let dist0 = (tri.p0.x - x).abs() + (tri.p0.y - y).abs();
    let dist1 = (tri.p1.x - x).abs() + (tri.p1.y - y).abs();
    let dist2 = (tri.p2.x - x).abs() + (tri.p2.y - y).abs();

    let min_dist = dist0.min(dist1).min(dist2);
    if min_dist < 2 {
        let raw = base.to_u32();
        let mut r = ((raw >> 24) & 0xFF) as i32;
        let mut g = ((raw >> 16) & 0xFF) as i32;
        let mut b = ((raw >> 8) & 0xFF) as i32;
        r = (r - 16).max(0);
        g = (g - 16).max(0);
        b = (b - 16).max(0);
        let a = (raw & 0xFF) as u8;
        return Rgba8888::rgba(r as u8, g as u8, b as u8, a);
    }
    base
}

fn draw_filled_triangle<R: Rasterizer>(
    rasterizer: &mut R,
    original: ScreenTriangle,
    color: Rgba8888,
    options: &RenderOptions,
) {
    let mut pts = [original.p0, original.p1, original.p2];
    pts.sort_by_key(|p| p.y);
    let (top, mid, bottom) = (pts[0], pts[1], pts[2]);

    if top.y == bottom.y {
        return;
    }

    let width = rasterizer.width() as i32;
    let height = rasterizer.height() as i32;

    let interp = |y: i32, y0: i32, y1: i32, x0: i32, x1: i32| {
        if y1 == y0 {
            x0
        } else {
            x0 + ((x1 - x0) * (y - y0)) / (y1 - y0)
        }
    };

    for y in top.y..=bottom.y {
        if y < 0 || y >= height {
            continue;
        }

        let (xa, xb) = if y < mid.y {
            (
                interp(y, top.y, bottom.y, top.x, bottom.x),
                interp(y, top.y, mid.y, top.x, mid.x),
            )
        } else {
            (
                interp(y, top.y, bottom.y, top.x, bottom.x),
                interp(y, mid.y, bottom.y, mid.x, bottom.x),
            )
        };

        let (x_start, x_end) = if xa <= xb { (xa, xb) } else { (xb, xa) };

        for x in x_start..=x_end {
            if x < 0 || x >= width {
                continue;
            }
            let mut final_color = color;
            if options.enable_antialiasing {
                final_color = apply_antialiasing(color, x, y, &original);
            }
            rasterizer.blend_pixel(x, y, final_color, 255);
        }
    }

    let mut min_x = original.p0.x.min(original.p1.x.min(original.p2.x));
    let mut max_x = original.p0.x.max(original.p1.x.max(original.p2.x));
    let mut min_y = original.p0.y.min(original.p1.y.min(original.p2.y));
    let mut max_y = original.p0.y.max(original.p1.y.max(original.p2.y));

    min_x = min_x.max(0);
    min_y = min_y.max(0);
    max_x = (max_x + 1).min(width);
    max_y = (max_y + 1).min(height);

    if min_x < max_x && min_y < max_y {
        rasterizer.mark_dirty(min_x, min_y, max_x, max_y);
    }
}

fn draw_wireframe_line<R: Rasterizer>(
    rasterizer: &mut R,
    p0: ScreenPoint,
    p1: ScreenPoint,
    color: Rgba8888,
) {
    let width = rasterizer.width() as i32;
    let height = rasterizer.height() as i32;

    let mut x0 = p0.x;
    let mut y0 = p0.y;
    let x1 = p1.x;
    let y1 = p1.y;

    let dx = (x1 - x0).abs();
    let sx = if x0 < x1 { 1 } else { -1 };
    let dy = -(y1 - y0).abs();
    let sy = if y0 < y1 { 1 } else { -1 };
    let mut err = dx + dy;

    loop {
        if x0 >= 0 && x0 < width && y0 >= 0 && y0 < height {
            rasterizer.blend_pixel(x0, y0, color, 255);
        }
        if x0 == x1 && y0 == y1 {
            break;
        }
        let e2 = err * 2;
        if e2 >= dy {
            err += dy;
            x0 += sx;
        }
        if e2 <= dx {
            err += dx;
            y0 += sy;
        }
    }

    let min_x = p0.x.min(p1.x).max(0);
    let min_y = p0.y.min(p1.y).max(0);
    let max_x = (p0.x.max(p1.x) + 1).min(width);
    let max_y = (p0.y.max(p1.y) + 1).min(height);

    if min_x < max_x && min_y < max_y {
        rasterizer.mark_dirty(min_x, min_y, max_x, max_y);
    }
}

/// Draws a 3D model onto the provided rasterizer.
pub fn draw_model<R: Rasterizer>(
    rasterizer: &mut R,
    model: &Model,
    origin: Vec3,
    rotation: Quaternion,
    options: &RenderOptions,
) {
    let light_dir = options.light_dir.normalize();
    let mut world_vertices = [Vec3(0.0, 0.0, 0.0); MAX_VERTICES];
    let mut projected = [None; MAX_VERTICES];

    let width = rasterizer.width() as u32;
    let height = rasterizer.height() as u32;

    for i in 0..model.vertex_count {
        let rotated = rotation.rotate_vector(model.vertices[i]);
        let world = Vec3(
            origin.0 + rotated.0,
            origin.1 + rotated.1,
            origin.2 + rotated.2,
        );
        world_vertices[i] = world;
        projected[i] = project(
            world,
            options.fov_deg,
            width,
            height,
            options.enable_near_clipping,
            options.enable_frustum_clipping,
        );
    }

    let mut triangle_meta = [(0usize, 0.0f32); MAX_TRIANGLES];
    for i in 0..model.triangle_count {
        let tri = &model.triangles[i];
        let z0 = world_vertices[tri.vertices[0]].2;
        let z1 = world_vertices[tri.vertices[1]].2;
        let z2 = world_vertices[tri.vertices[2]].2;
        triangle_meta[i] = (i, (z0 + z1 + z2) / 3.0);
    }

    if options.enable_depth_sorting {
        triangle_meta[..model.triangle_count]
            .sort_unstable_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(core::cmp::Ordering::Equal));
    }

    for &(tri_idx, _) in triangle_meta.iter().take(model.triangle_count) {
        let tri = &model.triangles[tri_idx];

        if let (Some(p0), Some(p1), Some(p2)) = (
            projected[tri.vertices[0]],
            projected[tri.vertices[1]],
            projected[tri.vertices[2]],
        ) {
            if options.enable_backface_culling {
                let edge1 = world_vertices[tri.vertices[1]].sub(world_vertices[tri.vertices[0]]);
                let edge2 = world_vertices[tri.vertices[2]].sub(world_vertices[tri.vertices[0]]);
                let normal = Vec3(
                    edge1.1 * edge2.2 - edge1.2 * edge2.1,
                    edge1.2 * edge2.0 - edge1.0 * edge2.2,
                    edge1.0 * edge2.1 - edge1.1 * edge2.0,
                )
                .normalize();
                if normal.dot(Vec3(0.0, 0.0, -1.0)) <= 0.0 {
                    continue;
                }
            }

            let diffuse = if options.enable_lighting {
                let rotated_normal = rotation.rotate_vector(tri.normal);
                rotated_normal.dot(light_dir).max(0.0)
            } else {
                1.0
            };

            let intensity = options.intensity_range.0
                + (options.intensity_range.1 - options.intensity_range.0) * diffuse;
            let color = get_grayscale_color(intensity);
            let screen_tri = ScreenTriangle { p0, p1, p2 };

            if options.enable_shading {
                draw_filled_triangle(rasterizer, screen_tri, color, options);
            }

            if options.enable_wireframe {
                let wire_color = Rgba8888::rgba(200, 200, 200, 255);
                draw_wireframe_line(rasterizer, p0, p1, wire_color);
                draw_wireframe_line(rasterizer, p1, p2, wire_color);
                draw_wireframe_line(rasterizer, p2, p0, wire_color);
            }
        }
    }
}
