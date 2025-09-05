#![no_std]

use crate::libs::gfx::math::{Quaternion, Vec3};
use crate::libs::gfx::two_d::types::{Point, Rgb565};
use crate::libs::gfx::two_d::{draw_line_aa, Rasterizer};
use crate::libs::gfx::Model;
use crate::system::kernel::config::resources::{MAX_TRIANGLES, MAX_VERTICES};
use core::cmp::Ordering;
use embedded_graphics_core::prelude::RgbColor;
use micromath::F32Ext;
use defmt::debug;
use embassy_time::Instant;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ShadingMode { Flat, Gouraud }

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum AntiAliasing { None, Edge }

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ViewMode { Fill, Wireframe, FillAndWireframe }

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum LightingMode { None, Ambient, Directional, AmbientAndDirectional }

/// Options controlling the software renderer.
///
/// These fields configure projection, lighting, rasterization behavior, and quality/perf toggles.
#[derive(Clone, Copy)]
pub struct RenderOptions {
    /// Vertical field-of-view in degrees for perspective projection.
    pub fov_deg: f32,
    /// Directional light direction in camera space.
    /// Camera looks along +Z; vectors with negative Z light faces oriented to the camera.
    pub light_dir: Vec3,
    /// Grayscale intensity range for lit fragments (min, max).
    /// Used by grayscale utilities; colored lighting ignores this directly.
    pub intensity_range: (f32, f32),
    /// Reject back-facing triangles.
    pub enable_backface_culling: bool,
    /// Enable painter's depth sorting by triangle centroid Z.
    /// This is a coarse visibility approximation without a z-buffer.
    pub enable_depth_sorting: bool,
    /// Lighting mode selection (None, Ambient, Directional, Both).
    pub lighting_mode: LightingMode,
    /// Clip geometry with a near plane at z = near_z.
    /// Triangles intersecting the plane are split in camera space.
    pub enable_near_clipping: bool,
    /// Discard projected points outside viewport bounds.
    /// Cheap bounds cull; does not perform full frustum clipping.
    pub enable_frustum_clipping: bool,
    /// View mode: fill, wireframe, or both.
    pub view_mode: ViewMode,
    /// Near plane distance. Only used when `enable_near_clipping` is true.
    pub near_z: f32,
    /// Shading mode: flat or Gouraud.
    pub shading_mode: ShadingMode,
    /// Anti-aliasing mode.
    /// Blends subpixel coverage on scanline edges (slower; reduces jaggies and seam visibility).
    pub aa_mode: AntiAliasing,
    /// Ambient light color (per-channel 5/6/5 expanded to 8-bit internally).
    pub ambient_color: Rgb565,
    /// Directional light color (per-channel 5/6/5 expanded to 8-bit internally).
    pub directional_color: Rgb565,
    /// Base model color (per-channel 5/6/5 expanded to 8-bit internally).
    /// Final color is: model × (ambient + lambert × directional).
    pub model_color: Rgb565,
}

/// Rendering statistics collected during a single draw call.
#[derive(Clone, Copy, Default)]
pub struct RenderStats {
    /// Total time in microseconds spent in draw_model (if timing enabled).
    pub micros_total: u64,
    /// Time spent transforming vertices to camera space.
    pub micros_transform: u64,
    /// Time spent projecting vertices.
    pub micros_project: u64,
    /// Time spent depth-sorting triangles.
    pub micros_sort: u64,
    /// Time spent clipping, shading, and rasterizing.
    pub micros_clip_raster: u64,
    /// Triangles processed from the model.
    pub triangles_input: u32,
    /// Triangles culled by backface test.
    pub triangles_culled: u32,
    /// Triangles produced after near-plane clipping (sum of emitted fans).
    pub triangles_emitted: u32,
    /// Pixels written via set_pixel (solid interior).
    pub pixels_filled: u32,
    /// Pixels blended via blend_pixel (edge AA).
    pub pixels_blended: u32,
    /// Wireframe edges drawn (segments).
    pub edges_drawn: u32,
}

#[inline(always)]
fn grayscale(intensity: f32) -> Rgb565 {
    let clamped = intensity.clamp(0.0, 1.0);
    let v8 = (clamped * 255.0).round() as u8;
    Rgb565::from_rgb(v8, v8, v8)
}

#[inline(always)]
fn rgb565_to_rgb8(c: Rgb565) -> (u8, u8, u8) {
    let r5 = ((c.0 >> 11) & 0x1F) as u8;
    let g6 = ((c.0 >> 5) & 0x3F) as u8;
    let b5 = (c.0 & 0x1F) as u8;
    let r = ((r5 as u16 * 527 + 23) >> 6) as u8;
    let g = ((g6 as u16 * 259 + 33) >> 6) as u8;
    let b = ((b5 as u16 * 527 + 23) >> 6) as u8;
    (r, g, b)
}

#[inline(always)]
fn modulate_lit_color(model: Rgb565, ambient: Rgb565, directional: Rgb565, lambert: f32) -> Rgb565 {
    let (mr, mg, mb) = rgb565_to_rgb8(model);
    let (ar, ag, ab) = rgb565_to_rgb8(ambient);
    let (dr, dg, db) = rgb565_to_rgb8(directional);
    let i = lambert.clamp(0.0, 1.0);
    let fr = (ar as f32 / 255.0) + (dr as f32 / 255.0) * i;
    let fg = (ag as f32 / 255.0) + (dg as f32 / 255.0) * i;
    let fb = (ab as f32 / 255.0) + (db as f32 / 255.0) * i;
    let rr = ((mr as f32) * fr).clamp(0.0, 255.0) as u8;
    let rg = ((mg as f32) * fg).clamp(0.0, 255.0) as u8;
    let rb = ((mb as f32) * fb).clamp(0.0, 255.0) as u8;
    Rgb565::from_rgb(rr, rg, rb)
}

/// Perspective projection from camera space (camera at origin looking +Z).
#[inline(always)]
fn project_perspective_precomputed(v: Vec3, f: f32, aspect: f32, w: u32, h: u32, near_z: f32, clip_near: bool, clip_bounds: bool) -> Option<Point> {
    if clip_near && v.2 <= near_z { return None; }
    // Camera looks along +Z; perspective divide by z
    let x_ndc = (v.0 * f) / (v.2 * aspect);
    let y_ndc = (v.1 * f) / v.2;

    let x = ((x_ndc + 1.0) * (w as f32) * 0.5) as i32;
    let y = ((1.0 - y_ndc) * (h as f32) * 0.5) as i32;

    if clip_bounds && (x < 0 || x >= w as i32 || y < 0 || y >= h as i32) { return None; }
    Some(Point::new(x, y))
}

#[inline(always)]
fn project_perspective(v: Vec3, fov_deg: f32, w: u32, h: u32, near_z: f32, clip_near: bool, clip_bounds: bool) -> Option<Point> {
    let fov_rad = fov_deg.to_radians();
    let f = 1.0 / (fov_rad * 0.5).tan();
    let aspect = w as f32 / h as f32;
    project_perspective_precomputed(v, f, aspect, w, h, near_z, clip_near, clip_bounds)
}

#[derive(Clone, Copy)]
struct FlatTriangle { p0: Point, p1: Point, p2: Point, color: Rgb565 }

#[derive(Clone, Copy)]
struct GouraudTriangle { p0: Point, p1: Point, p2: Point, i0: f32, i1: f32, i2: f32 }

/// Simple scanline triangle filler.
fn fill_triangle<R: Rasterizer>(r: &mut R, tri: &FlatTriangle, aa: bool) {
    let mut pts = [tri.p0, tri.p1, tri.p2];
    pts.sort_by_key(|p| p.y);
    let (top, mid, bot) = (pts[0], pts[1], pts[2]);
    if top.y == bot.y { return; }
    let w = r.width() as i32;
    let h = r.height() as i32;
    let interp = |y, y0, y1, x0, x1| if y1 == y0 { x0 } else { x0 + ((x1 - x0) * (y - y0)) / (y1 - y0) };
    let interp_f = |y: i32, y0: i32, y1: i32, x0: i32, x1: i32| if y1 == y0 { x0 as f32 } else { x0 as f32 + (x1 - x0) as f32 * (y - y0) as f32 / (y1 - y0) as f32 };
    for y in top.y..=bot.y {
        if y < 0 || y >= h { continue; }
        let (xa, xb, xa_f, xb_f) = if y < mid.y {
            (
                interp(y, top.y, bot.y, top.x, bot.x),
                interp(y, top.y, mid.y, top.x, mid.x),
                interp_f(y, top.y, bot.y, top.x, bot.x),
                interp_f(y, top.y, mid.y, top.x, mid.x),
            )
        } else {
            (
                interp(y, top.y, bot.y, top.x, bot.x),
                interp(y, mid.y, bot.y, mid.x, bot.x),
                interp_f(y, top.y, bot.y, top.x, bot.x),
                interp_f(y, mid.y, bot.y, mid.x, bot.x),
            )
        };
        let (x_start, x_end, x_start_f, x_end_f) = if xa <= xb { (xa, xb, xa_f, xb_f) } else { (xb, xa, xb_f, xa_f) };
        let xs = x_start.max(0); let xe = x_end.min(w - 1);
        if xs <= xe {
            if aa {
                // Subpixel edge coverage for start and end pixels
                if xs >= 0 && xs < w {
                    let cov_start = 1.0 - (x_start_f.fract()).abs();
                    let alpha = (cov_start.clamp(0.0, 1.0) * 255.0) as u8;
                    r.blend_pixel(xs, y, tri.color, alpha);
                }
                if xe >= 0 && xe < w && xe != xs {
                    let cov_end = (x_end_f.fract()).abs();
                    let alpha = (cov_end.clamp(0.0, 1.0) * 255.0) as u8;
                    r.blend_pixel(xe, y, tri.color, alpha);
                }
                // Fill inner (exclusive of edges)
                let inner_start = (xs + 1).min(xe);
                for x in inner_start..xe { r.set_pixel(x, y, tri.color); }
            } else {
                // Solid fill inclusive
                for x in xs..=xe { r.set_pixel(x, y, tri.color); }
            }
        }
    }
}

/// Gouraud scanline fill (interpolate intensity along edges and across span).
fn fill_triangle_gouraud<R: Rasterizer>(r: &mut R, tri: &GouraudTriangle, range: (f32, f32), model: Rgb565, ambient: Rgb565, directional: Rgb565, aa: bool) {
    let mut pts = [(tri.p0, tri.i0), (tri.p1, tri.i1), (tri.p2, tri.i2)];
    pts.sort_by_key(|p| p.0.y);
    let (top, mid, bot) = (pts[0], pts[1], pts[2]);
    if top.0.y == bot.0.y { return; }
    let w = r.width() as i32;
    let h = r.height() as i32;
    let interp = |y, y0, y1, x0, x1| if y1 == y0 { x0 } else { x0 + ((x1 - x0) * (y - y0)) / (y1 - y0) };
    let interpf = |y: i32, y0: i32, y1: i32, v0: f32, v1: f32| if y1 == y0 { v0 } else { v0 + (v1 - v0) * ((y - y0) as f32) / ((y1 - y0) as f32) };
    for y in top.0.y..=bot.0.y {
        if y < 0 || y >= h { continue; }
        let (xa, ia, xb, ib) = if y < mid.0.y {
            (
                interp(y, top.0.y, bot.0.y, top.0.x, bot.0.x),
                interpf(y, top.0.y, bot.0.y, top.1, bot.1),
                interp(y, top.0.y, mid.0.y, top.0.x, mid.0.x),
                interpf(y, top.0.y, mid.0.y, top.1, mid.1),
            )
        } else {
            (
                interp(y, top.0.y, bot.0.y, top.0.x, bot.0.x),
                interpf(y, top.0.y, bot.0.y, top.1, bot.1),
                interp(y, mid.0.y, bot.0.y, mid.0.x, bot.0.x),
                interpf(y, mid.0.y, bot.0.y, mid.1, bot.1),
            )
        };
        let (x_start, x_end, i_start, i_end) = if xa <= xb { (xa, xb, ia, ib) } else { (xb, xa, ib, ia) };
        let xs = x_start.max(0); let xe = x_end.min(w - 1);
        if xs <= xe {
            if aa {
                // Blend AA at edges
                if xs >= 0 && xs < w {
                    let i = i_start.clamp(0.0, 1.0);
                    let color = modulate_lit_color(model, ambient, directional, i);
                    r.blend_pixel(xs, y, color, 160);
                }
                if xe >= 0 && xe < w && xe != xs {
                    let i = i_end.clamp(0.0, 1.0);
                    let color = modulate_lit_color(model, ambient, directional, i);
                    r.blend_pixel(xe, y, color, 160);
                }
                let inner_start = (xs + 1).min(xe);
                for x in inner_start..xe {
                    let t = if x_end == x_start { 0.0 } else { (x - x_start) as f32 / (x_end - x_start) as f32 };
                    let i = (i_start + (i_end - i_start) * t).clamp(0.0, 1.0);
                    let color = modulate_lit_color(model, ambient, directional, i);
                    r.set_pixel(x, y, color);
                }
            } else {
                for x in xs..=xe {
                    let t = if x_end == x_start { 0.0 } else { (x - x_start) as f32 / (x_end - x_start) as f32 };
                    let i = (i_start + (i_end - i_start) * t).clamp(0.0, 1.0);
                    let color = modulate_lit_color(model, ambient, directional, i);
                    r.set_pixel(x, y, color);
                }
            }
        }
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

fn compute_vertex_normals_cam_space(model: &Model, vertices_cam: &[Vec3; MAX_VERTICES]) -> [Vec3; MAX_VERTICES] {
    let mut normals = [Vec3(0.0, 0.0, 0.0); MAX_VERTICES];
    for i in 0..model.triangle_count {
        let tri = &model.triangles[i];
        let n = face_normal_cam_space(
            vertices_cam[tri.vertices[0]],
            vertices_cam[tri.vertices[1]],
            vertices_cam[tri.vertices[2]],
        );
        // Accumulate face normal to each vertex normal
        for &vi in &tri.vertices {
            normals[vi] = normals[vi].add(n);
        }
    }
    for i in 0..model.vertex_count { normals[i] = normals[i].normalize(); }
    normals
}

pub fn draw_model<R: Rasterizer>(raster: &mut R, model: &Model, origin_cam: Vec3, model_rotation: Quaternion, width: u32, height: u32, options: &RenderOptions) {
    let start_time = Instant::now();
    let light_dir = options.light_dir.normalize();
    let fov_rad = options.fov_deg.to_radians();
    let f = 1.0 / (fov_rad * 0.5).tan();
    let aspect = width as f32 / height as f32;

    // Camera-space transform (model -> camera).
    let transform_start = Instant::now();
    let vertices_cam = transform_vertices(model, origin_cam, model_rotation);
    let transform_time = transform_start.elapsed().as_micros();
    
    let project_start = Instant::now();
    let projected = project_vertices(&vertices_cam, options.fov_deg, width, height, options.near_z, options.enable_near_clipping, options.enable_frustum_clipping);
    let project_time = project_start.elapsed().as_micros();
    
    let vertex_normals_cam = if matches!(options.shading_mode, ShadingMode::Gouraud) || !matches!(options.lighting_mode, LightingMode::None) {
        compute_vertex_normals_cam_space(model, &vertices_cam)
    } else { [Vec3(0.0, 0.0, 0.0); MAX_VERTICES] };

    // Painter's algorithm ordering (optional).
    let mut order = triangle_order(model, &vertices_cam);
    let sort_start = Instant::now();
    if options.enable_depth_sorting {
        order[..model.triangle_count].sort_unstable_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(Ordering::Equal));
    }
    let sort_time = sort_start.elapsed().as_micros();

    let raster_start = Instant::now();
    let mut triangles_culled = 0u32;
    let mut triangles_emitted = 0u32;
    let mut pixels_filled = 0u32;
    let mut pixels_blended = 0u32;
    let mut edges_drawn = 0u32;
    for &(tri_idx, _) in order.iter().take(model.triangle_count) {
        let tri = &model.triangles[tri_idx];
        let i0 = tri.vertices[0];
        let i1 = tri.vertices[1];
        let i2 = tri.vertices[2];

        // Backface culling in camera space: camera looks +Z, front faces have normal.z < 0.
        if options.enable_backface_culling {
            let n = face_normal_cam_space(vertices_cam[i0], vertices_cam[i1], vertices_cam[i2]);
            if n.2 >= 0.0 { 
                triangles_culled += 1;
                continue; 
            }
        }

        // Prepare camera-space vertices for clipping and shading
        let mut cam = [vertices_cam[i0], vertices_cam[i1], vertices_cam[i2]];
        let mut intens = [0.0f32; 3];
        if !matches!(options.lighting_mode, LightingMode::None) {
            if matches!(options.shading_mode, ShadingMode::Gouraud) {
                let n0 = vertex_normals_cam[i0];
                let n1 = vertex_normals_cam[i1];
                let n2 = vertex_normals_cam[i2];
                intens = [n0.dot(light_dir).max(0.0), n1.dot(light_dir).max(0.0), n2.dot(light_dir).max(0.0)];
            } else {
                let n = face_normal_cam_space(cam[0], cam[1], cam[2]);
                let l = n.dot(light_dir).max(0.0);
                intens = [l, l, l];
            }
        } else {
            intens = [1.0, 1.0, 1.0];
        }

        // Near-plane clipping (z >= near_z)
        let near_z = options.near_z;
        let inside = |v: Vec3| v.2 >= near_z;
        let intersect = |a: Vec3, b: Vec3, ia: f32, ib: f32| {
            let t = if (b.2 - a.2) == 0.0 { 0.0 } else { (near_z - a.2) / (b.2 - a.2) };
            let p = Vec3(
                a.0 + (b.0 - a.0) * t,
                a.1 + (b.1 - a.1) * t,
                near_z,
            );
            let i = ia + (ib - ia) * t;
            (p, i)
        };

        // Sutherland–Hodgman for a single plane
        let mut pts: [(Vec3, f32); 5] = [(cam[0], intens[0]), (cam[1], intens[1]), (cam[2], intens[2]), (Vec3(0.0,0.0,0.0),0.0), (Vec3(0.0,0.0,0.0),0.0)];
        let mut out: [(Vec3, f32); 5] = [(Vec3(0.0,0.0,0.0),0.0); 5];
        let mut out_len = 0usize;
        for e in 0..3 {
            let curr = pts[e];
            let next = pts[(e + 1) % 3];
            let curr_in = inside(curr.0);
            let next_in = inside(next.0);
            match (curr_in, next_in) {
                (true, true) => {
                    out[out_len] = next; out_len += 1;
                }
                (true, false) => {
                    let (p, i) = intersect(curr.0, next.0, curr.1, next.1);
                    out[out_len] = (p, i); out_len += 1;
                }
                (false, true) => {
                    let (p, i) = intersect(curr.0, next.0, curr.1, next.1);
                    out[out_len] = (p, i); out_len += 1;
                    out[out_len] = next; out_len += 1;
                }
                (false, false) => {}
            }
        }
        if out_len < 3 { continue; }

        // Triangulate fan (can be triangle or quad -> two triangles)
        let mut draw_poly = |a: (Vec3,f32), b: (Vec3,f32), c: (Vec3,f32)| {
            if let (Some(p0), Some(p1), Some(p2)) = (
                project_perspective_precomputed(a.0, f, aspect, width, height, options.near_z, options.enable_near_clipping, options.enable_frustum_clipping),
                project_perspective_precomputed(b.0, f, aspect, width, height, options.near_z, options.enable_near_clipping, options.enable_frustum_clipping),
                project_perspective_precomputed(c.0, f, aspect, width, height, options.near_z, options.enable_near_clipping, options.enable_frustum_clipping),
            ) {
                if matches!(options.shading_mode, ShadingMode::Gouraud) && !matches!(options.lighting_mode, LightingMode::None) {
                    fill_triangle_gouraud(
                        raster,
                        &GouraudTriangle { p0, p1, p2, i0: a.1, i1: b.1, i2: c.1 },
                        options.intensity_range,
                        options.model_color,
                        options.ambient_color,
                        options.directional_color,
                        matches!(options.aa_mode, AntiAliasing::Edge),
                    );
                } else {
                    let lambert = a.1; // flat path: all equal
                    let (amb_on, dir_on) = match options.lighting_mode {
                        LightingMode::None => (false, false),
                        LightingMode::Ambient => (true, false),
                        LightingMode::Directional => (false, true),
                        LightingMode::AmbientAndDirectional => (true, true),
                    };
                    let color = if amb_on || dir_on {
                        let lambert_eff = if dir_on { lambert } else { 0.0 };
                        let amb = if amb_on { options.ambient_color } else { Rgb565::from_rgb(0,0,0) };
                        modulate_lit_color(options.model_color, amb, options.directional_color, lambert_eff)
                    } else { options.model_color };
                    if matches!(options.view_mode, ViewMode::Fill | ViewMode::FillAndWireframe) {
                        fill_triangle(raster, &FlatTriangle { p0, p1, p2, color }, matches!(options.aa_mode, AntiAliasing::Edge));
                    }
                }
                if matches!(options.view_mode, ViewMode::Wireframe | ViewMode::FillAndWireframe) {
                    draw_line_aa(raster, p0, p1, Rgb565::from_rgb(0, 255, 0));
                    draw_line_aa(raster, p1, p2, Rgb565::from_rgb(0, 255, 0));
                    draw_line_aa(raster, p2, p0, Rgb565::from_rgb(0, 255, 0));
                    edges_drawn += 3;
                }
                triangles_emitted += 1;
            }
        };

        for k in 1..(out_len - 1) {
            draw_poly(out[0], out[k], out[k + 1]);
        }
    }
    
    let raster_time = raster_start.elapsed().as_micros();
    let total_time = start_time.elapsed().as_micros();
    
    debug!(
        "3D render: total={}us transform={}us project={}us sort={}us raster={}us tri_in={} tri_culled={} tri_out={} edges={}",
        total_time,
        transform_time,
        project_time,
        sort_time,
        raster_time,
        model.triangle_count,
        triangles_culled,
        triangles_emitted,
        edges_drawn
    );
}
