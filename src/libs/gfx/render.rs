// render.rs
#![no_std]

use core::iter::Iterator;
use embedded_graphics::{
    pixelcolor::Gray4,
    prelude::{DrawTarget, Dimensions, OriginDimensions, Point, Size},
    Drawable, Pixel,
};
use embedded_graphics::prelude::Primitive;
use embedded_graphics::primitives::{Triangle, Line, PrimitiveStyle};
use embedded_graphics_core::prelude::GrayColor;
use micromath::F32Ext;

use super::math::{Quaternion, Vec3};
use super::model::{Model, MAX_TRIANGLES, MAX_VERTICES};

/// Configuration structure for rendering the 3D model.
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
fn get_grayscale_color(intensity: f32) -> Gray4 {
    let clamped = intensity.clamp(0.0, 1.0);
    let value = (clamped * 15.0).round() as u8;
    Gray4::new(value)
}

#[inline(always)]
fn project(v: Vec3, fov_deg: f32, width: u32, height: u32, clip_near: bool, clip_bounds: bool) -> Option<Point> {
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

    Some(Point::new(x, y))
}

struct ShadedTriangle {
    p0: Point,
    p1: Point,
    p2: Point,
    color: Gray4,
}

impl OriginDimensions for ShadedTriangle {
    fn size(&self) -> Size {
        Triangle::new(self.p0, self.p1, self.p2).bounding_box().size
    }
}

impl Drawable for ShadedTriangle {
    type Color = Gray4;
    type Output = ();

    fn draw<D: DrawTarget<Color = Gray4>>(&self, target: &mut D) -> Result<(), D::Error> {
        let mut pts = [self.p0, self.p1, self.p2];
        pts.sort_by_key(|p| p.y);
        let (top, mid, bot) = (pts[0], pts[1], pts[2]);

        if top.y == bot.y {
            return Ok(());
        }

        let w = target.bounding_box().size.width as i32;
        let h = target.bounding_box().size.height as i32;

        let interp = |y, y0, y1, x0, x1| {
            if y1 == y0 {
                x0
            } else {
                x0 + ((x1 - x0) * (y - y0)) / (y1 - y0)
            }
        };

        for y in top.y..=bot.y {
            if y < 0 || y >= h {
                continue;
            }

            let (xa, xb) = if y < mid.y {
                (
                    interp(y, top.y, bot.y, top.x, bot.x),
                    interp(y, top.y, mid.y, top.x, mid.x),
                )
            } else {
                (
                    interp(y, top.y, bot.y, top.x, bot.x),
                    interp(y, mid.y, bot.y, mid.x, bot.x),
                )
            };

            let (x_start, x_end) = if xa < xb { (xa, xb) } else { (xb, xa) };

            for x in x_start..=x_end {
                if x >= 0 && x < w {
                    let mut final_color = self.color;

                    if let Some(antialiased_color) = apply_antialiasing(x, y, self) {
                        final_color = antialiased_color;
                    }

                    target.draw_iter(core::iter::once(Pixel(Point::new(x, y), final_color)))?;
                }
            }
        }
        Ok(())
    }
}

fn apply_antialiasing(x: i32, y: i32, tri: &ShadedTriangle) -> Option<Gray4> {
    let dist0 = (tri.p0.x - x).abs() + (tri.p0.y - y).abs();
    let dist1 = (tri.p1.x - x).abs() + (tri.p1.y - y).abs();
    let dist2 = (tri.p2.x - x).abs() + (tri.p2.y - y).abs();

    let min_dist = dist0.min(dist1).min(dist2);
    if min_dist < 2 {
        let mut val = tri.color.luma();
        if val > 0 {
            val -= 1;
        }
        return Some(Gray4::new(val));
    }
    None
}

pub fn draw_model<D: DrawTarget<Color = Gray4>>(
    display: &mut D,
    model: &Model,
    origin: Vec3,
    rotation: Quaternion,
    width: u32,
    height: u32,
    options: &RenderOptions,
) -> Result<(), D::Error> {
    let light_dir = options.light_dir.normalize();
    let mut world_vertices = [Vec3(0.0, 0.0, 0.0); MAX_VERTICES];
    let mut projected = [None; MAX_VERTICES];

    for i in 0..model.vertex_count {
        let rotated = rotation.rotate_vector(model.vertices[i]);
        let world = Vec3(origin.0 + rotated.0, origin.1 + rotated.1, origin.2 + rotated.2);
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
        triangle_meta[..model.triangle_count].sort_unstable_by(|a, b| {
            b.1.partial_cmp(&a.1).unwrap_or(core::cmp::Ordering::Equal)
        });
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

            if options.enable_shading {
                ShadedTriangle { p0, p1, p2, color }.draw(display)?;
            }

            if options.enable_wireframe {
                let style = PrimitiveStyle::with_stroke(Gray4::new(15), 1);
                Line::new(p0, p1).into_styled(style).draw(display)?;
                Line::new(p1, p2).into_styled(style).draw(display)?;
                Line::new(p2, p0).into_styled(style).draw(display)?;
            }
        }
    }

    Ok(())
}
