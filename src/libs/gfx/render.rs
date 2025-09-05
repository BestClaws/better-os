#![no_std]

use core::iter::Iterator;
use crate::libs::gfx::two_d::types::{Point, Rgb565};
use micromath::F32Ext;
use defmt::{info, debug, warn, error};
use embedded_graphics_core::prelude::RgbColor;
use crate::libs::gfx::Model;
use crate::libs::gfx::two_d::{draw_line_aa, Rasterizer};
use crate::system::kernel::config::resources::{MAX_TRIANGLES, MAX_VERTICES};
use super::math::{Quaternion, Vec3};

#[derive(Clone, Copy)]
pub struct RenderOptions {
    pub fov_deg: f32,
    pub light_dir: Vec3,
    pub intensity_range: (f32, f32),
    pub enable_backface_culling: bool,
    pub enable_zbuffer: bool,
    pub enable_lighting: bool,
    pub enable_depth_sorting: bool,
    pub enable_near_clipping: bool,
    pub enable_frustum_clipping: bool,
    pub enable_wireframe: bool,
    pub enable_shading: bool,
    pub enable_antialiasing: bool,
    pub antialiasing_factor: u8,
    pub edge_only_antialiasing: bool,
}

#[inline(always)]
fn get_grayscale_color(intensity: f32) -> Rgb565 {
    let clamped = intensity.clamp(0.0, 1.0);
    let v8 = (clamped * 255.0).round() as u8;
    Rgb565::from_rgb(v8, v8, v8)
}

#[inline(always)]
fn project(
    v: Vec3,
    fov_deg: f32,
    width: u32,
    height: u32,
    clip_near: bool,
    clip_bounds: bool,
) -> Option<Point> {
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
    color: Rgb565,
}

// Triangle fill using our rasterizer target
fn draw_triangle<R: Rasterizer>(r: &mut R, tri: &ShadedTriangle) {
    let mut pts = [tri.p0, tri.p1, tri.p2];
    pts.sort_by_key(|p| p.y);
    let (top, mid, bot) = (pts[0], pts[1], pts[2]);
    if top.y == bot.y { return; }
    let w = r.width() as i32;
    let h = r.height() as i32;
    let interp = |y, y0, y1, x0, x1| if y1 == y0 { x0 } else { x0 + ((x1 - x0) * (y - y0)) / (y1 - y0) };
    for y in top.y..=bot.y {
        if y < 0 || y >= h { continue; }
        let (xa, xb) = if y < mid.y {
            (interp(y, top.y, bot.y, top.x, bot.x), interp(y, top.y, mid.y, top.x, mid.x))
        } else {
            (interp(y, top.y, bot.y, top.x, bot.x), interp(y, mid.y, bot.y, mid.x, bot.x))
        };
        let (x_start, x_end) = if xa < xb { (xa, xb) } else { (xb, xa) };
        for x in x_start..=x_end {
            if x >= 0 && x < w { r.set_pixel(x, y, tri.color); }
        }
    }
}

fn apply_antialiasing(x: i32, y: i32, tri: &ShadedTriangle) -> Option<Rgb565> {
    let dist0 = (tri.p0.x - x).abs() + (tri.p0.y - y).abs();
    let dist1 = (tri.p1.x - x).abs() + (tri.p1.y - y).abs();
    let dist2 = (tri.p2.x - x).abs() + (tri.p2.y - y).abs();

    let min_dist = dist0.min(dist1).min(dist2);
    if min_dist < 2 {
        // Dim towards black for edge pixels to approximate AA
        return Some(tri.color.blend_over(Rgb565::BLACK, 128));
    }
    None
}

fn transform_and_project_vertices(
    model: &Model,
    origin: Vec3,
    rotation: Quaternion,
    width: u32,
    height: u32,
    options: &RenderOptions,
) -> ([Vec3; MAX_VERTICES], [Option<Point>; MAX_VERTICES]) {
    let mut world_vertices = [Vec3(0.0, 0.0, 0.0); MAX_VERTICES];
    let mut projected = [None; MAX_VERTICES];

    for i in 0..model.vertex_count {
        let rotated = rotation.rotate_vector(model.vertices[i]);
        let world = Vec3(origin.0 + rotated.0, origin.1 + rotated.1, origin.2 + rotated.2);
        world_vertices[i] = world;
        projected[i] = project(world, options.fov_deg, width, height, options.enable_near_clipping, options.enable_frustum_clipping);
    }

    (world_vertices, projected)
}

fn sort_triangles(model: &Model, world_vertices: &[Vec3; MAX_VERTICES]) -> [(usize, f32); MAX_TRIANGLES] {
    let mut triangle_meta = [(0usize, 0.0f32); MAX_TRIANGLES];
    for i in 0..model.triangle_count {
        let tri = &model.triangles[i];
        let z0 = world_vertices[tri.vertices[0]].2;
        let z1 = world_vertices[tri.vertices[1]].2;
        let z2 = world_vertices[tri.vertices[2]].2;
        triangle_meta[i] = (i, (z0 + z1 + z2) / 3.0);
    }
    triangle_meta
}

pub fn draw_model<R: Rasterizer>(
    raster: &mut R,
    model: &Model,
    origin: Vec3,
    rotation: Quaternion,
    width: u32,
    height: u32,
    options: &RenderOptions,
) {
    let light_dir = options.light_dir.normalize();

    let (world_vertices, projected) = transform_and_project_vertices(model, origin, rotation, width, height, options);

    let mut triangle_meta = sort_triangles(model, &world_vertices);
    if options.enable_depth_sorting {
        triangle_meta[..model.triangle_count].sort_unstable_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(core::cmp::Ordering::Equal));
    }

    for &(tri_idx, _) in triangle_meta.iter().take(model.triangle_count) {
        let tri = &model.triangles[tri_idx];

        if let (Some(p0), Some(p1), Some(p2)) = (projected[tri.vertices[0]], projected[tri.vertices[1]], projected[tri.vertices[2]]) {
            if options.enable_backface_culling {
                let edge1 = world_vertices[tri.vertices[1]].sub(world_vertices[tri.vertices[0]]);
                let edge2 = world_vertices[tri.vertices[2]].sub(world_vertices[tri.vertices[0]]);
                let normal = Vec3(
                    edge1.1 * edge2.2 - edge1.2 * edge2.1,
                    edge1.2 * edge2.0 - edge1.0 * edge2.2,
                    edge1.0 * edge2.1 - edge1.1 * edge2.0,
                ).normalize();
                let view_dir = Vec3(0.0, 0.0, -1.0);
                if normal.dot(view_dir) <= 0.0 { continue; }
            }

            let diffuse = if options.enable_lighting {
                let rotated_normal = rotation.rotate_vector(tri.normal);
                rotated_normal.dot(light_dir).max(0.0)
            } else { 1.0 };

            let intensity = options.intensity_range.0 + (options.intensity_range.1 - options.intensity_range.0) * diffuse;
            let color = get_grayscale_color(intensity);

            if options.enable_shading { draw_triangle(raster, &ShadedTriangle { p0, p1, p2, color }); }
            if options.enable_wireframe { draw_line_aa(raster, p0, p1, Rgb565::from_rgb(0, 255, 0)); draw_line_aa(raster, p1, p2, Rgb565::from_rgb(0, 255, 0)); draw_line_aa(raster, p2, p0, Rgb565::from_rgb(0, 255, 0)); }
        }
    }
}
