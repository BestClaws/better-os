#![no_std]

use crate::libs::gfx::math::{Quaternion, Vec3};
use crate::libs::gfx::two_d::types::{Point, Rgb565};
use crate::libs::gfx::two_d::{draw_line_aa, Rasterizer};
use crate::libs::gfx::Model;
use crate::system::kernel::config::resources::{MAX_TRIANGLES, MAX_VERTICES};
use core::cmp::Ordering;
use embedded_graphics_core::prelude::RgbColor;
use micromath::F32Ext;

/// Options controlling the software renderer.
#[derive(Clone, Copy)]
pub struct RenderOptions {
    /// Vertical field-of-view in degrees for perspective projection.
    pub fov_deg: f32,
    /// Directional light direction in camera space.
    pub light_dir: Vec3,
    /// Grayscale intensity range for lit fragments (min, max).
    pub intensity_range: (f32, f32),
    /// Reject back-facing triangles.
    pub enable_backface_culling: bool,
    /// Enable painter's depth sorting by triangle centroid Z.
    pub enable_depth_sorting: bool,
    /// Enable simple Lambert lighting.
    pub enable_lighting: bool,
    /// Clip geometry with a near plane at z = near_z.
    pub enable_near_clipping: bool,
    /// Discard projected points outside viewport bounds.
    pub enable_frustum_clipping: bool,
    /// Draw wireframe edges for debugging.
    pub enable_wireframe: bool,
    /// Fill triangles with flat shading.
    pub enable_shading: bool,
    /// Near plane distance. Only used when `enable_near_clipping` is true.
    pub near_z: f32,
}

#[inline(always)]
fn grayscale(intensity: f32) -> Rgb565 {
    let clamped = intensity.clamp(0.0, 1.0);
    let v8 = (clamped * 255.0).round() as u8;
    Rgb565::from_rgb(v8, v8, v8)
}

/// Perspective projection from camera space (camera at origin looking +Z).
#[inline(always)]
fn project_perspective(v: Vec3, fov_deg: f32, w: u32, h: u32, near_z: f32, clip_near: bool, clip_bounds: bool) -> Option<Point> {
    if clip_near && v.2 <= near_z { return None; }

    let fov_rad = fov_deg.to_radians();
    let f = 1.0 / (fov_rad * 0.5).tan();
    let aspect = w as f32 / h as f32;

    // Camera looks along +Z; perspective divide by z
    let x_ndc = (v.0 * f) / (v.2 * aspect);
    let y_ndc = (v.1 * f) / v.2;

    let x = ((x_ndc + 1.0) * (w as f32) * 0.5) as i32;
    let y = ((1.0 - y_ndc) * (h as f32) * 0.5) as i32;

    if clip_bounds && (x < 0 || x >= w as i32 || y < 0 || y >= h as i32) { return None; }
    Some(Point::new(x, y))
}

#[derive(Clone, Copy)]
struct FlatTriangle { p0: Point, p1: Point, p2: Point, color: Rgb565 }

/// Simple scanline triangle filler.
fn fill_triangle<R: Rasterizer>(r: &mut R, tri: &FlatTriangle) {
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
        let (x_start, x_end) = if xa <= xb { (xa, xb) } else { (xb, xa) };
        let xs = x_start.max(0); let xe = x_end.min(w - 1);
        for x in xs..=xe { r.set_pixel(x, y, tri.color); }
    }
}

#[inline(always)]
fn face_normal_cam_space(a: Vec3, b: Vec3, c: Vec3) -> Vec3 {
    let e1 = b.sub(a);
    let e2 = c.sub(a);
    e1.cross(e2).normalize()
}

fn transform_vertices(model: &Model, origin_cam: Vec3, model_rotation: Quaternion) -> [Vec3; MAX_VERTICES] {
    let mut cam_vertices = [Vec3(0.0, 0.0, 0.0); MAX_VERTICES];
    for i in 0..model.vertex_count {
        let rotated = model_rotation.rotate_vector(model.vertices[i]);
        cam_vertices[i] = Vec3(origin_cam.0 + rotated.0, origin_cam.1 + rotated.1, origin_cam.2 + rotated.2);
    }
    cam_vertices
}

fn project_vertices(vertices_cam: &[Vec3; MAX_VERTICES], fov_deg: f32, w: u32, h: u32, near_z: f32, clip_near: bool, clip_bounds: bool) -> [Option<Point>; MAX_VERTICES] {
    let mut projected = [None; MAX_VERTICES];
    for i in 0..vertices_cam.len() {
        if vertices_cam[i].length_squared() == 0.0 { continue; }
        projected[i] = project_perspective(vertices_cam[i], fov_deg, w, h, near_z, clip_near, clip_bounds);
    }
    projected
}

fn triangle_order(model: &Model, vertices_cam: &[Vec3; MAX_VERTICES]) -> [(usize, f32); MAX_TRIANGLES] {
    let mut order = [(0usize, 0.0f32); MAX_TRIANGLES];
    for i in 0..model.triangle_count {
        let t = &model.triangles[i];
        let z = (vertices_cam[t.vertices[0]].2 + vertices_cam[t.vertices[1]].2 + vertices_cam[t.vertices[2]].2) / 3.0;
        order[i] = (i, z);
    }
    order
}

pub fn draw_model<R: Rasterizer>(raster: &mut R, model: &Model, origin_cam: Vec3, model_rotation: Quaternion, width: u32, height: u32, options: &RenderOptions) {
    let light_dir = options.light_dir.normalize();

    // Camera-space transform (model -> camera).
    let vertices_cam = transform_vertices(model, origin_cam, model_rotation);
    let projected = project_vertices(&vertices_cam, options.fov_deg, width, height, options.near_z, options.enable_near_clipping, options.enable_frustum_clipping);

    // Painter's algorithm ordering (optional).
    let mut order = triangle_order(model, &vertices_cam);
    if options.enable_depth_sorting {
        order[..model.triangle_count].sort_unstable_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(Ordering::Equal));
    }

    for &(tri_idx, _) in order.iter().take(model.triangle_count) {
        let tri = &model.triangles[tri_idx];
        let i0 = tri.vertices[0];
        let i1 = tri.vertices[1];
        let i2 = tri.vertices[2];

        // Backface culling in camera space: camera looks +Z, front faces have normal.z < 0.
        if options.enable_backface_culling {
            let n = face_normal_cam_space(vertices_cam[i0], vertices_cam[i1], vertices_cam[i2]);
            if n.2 >= 0.0 { continue; }
        }

        // Projected points must all be valid to draw.
        let (Some(p0), Some(p1), Some(p2)) = (projected[i0], projected[i1], projected[i2]) else { continue };

        // Flat Lambert lighting using face normal in camera space.
        let color = if options.enable_lighting {
            let n = face_normal_cam_space(vertices_cam[i0], vertices_cam[i1], vertices_cam[i2]);
            let lambert = (-n.dot(light_dir)).max(0.0); // negative because front faces have n.z < 0
            let intensity = options.intensity_range.0 + (options.intensity_range.1 - options.intensity_range.0) * lambert;
            grayscale(intensity)
        } else { grayscale(1.0) };

        if options.enable_shading { fill_triangle(raster, &FlatTriangle { p0, p1, p2, color }); }
        if options.enable_wireframe {
            draw_line_aa(raster, p0, p1, Rgb565::from_rgb(0, 255, 0));
            draw_line_aa(raster, p1, p2, Rgb565::from_rgb(0, 255, 0));
            draw_line_aa(raster, p2, p0, Rgb565::from_rgb(0, 255, 0));
        }
    }
}
