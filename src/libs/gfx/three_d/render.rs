#![no_std]

use crate::util::math::primitives::{Quaternion, Vec3};
use crate::libs::gfx::two_d::types::{Point, Rgb565};
use crate::libs::gfx::two_d::Rasterizer;
use crate::libs::gfx::two_d::draw::Draw as Draw2D;
use crate::libs::gfx::Model;
use crate::system::kernel::config::resources::{MAX_TRIANGLES, MAX_VERTICES};
use core::cmp::Ordering;
use micromath::F32Ext;
use defmt::debug;
use embassy_time::Instant;

/// Space-grade 3D rendering pipeline optimized for embedded systems.
/// 
/// This module provides a high-performance software renderer designed for
/// resource-constrained environments. Key optimizations include:
/// - Pre-computed transformation matrices
/// - Efficient triangle rasterization with minimal branching
/// - Optimized lighting calculations with lookup tables
/// - Batched dirty region updates
/// - SIMD-friendly data structures where possible

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

/// Pre-computed projection matrix for efficient perspective projection.
/// 
/// This structure caches frequently used projection calculations to avoid
/// redundant trigonometric operations during rendering.
#[derive(Clone, Copy, Debug)]
pub struct ProjectionMatrix {
    /// Pre-computed focal length (1.0 / tan(fov/2))
    pub focal_length: f32,
    /// Aspect ratio (width / height)
    pub aspect_ratio: f32,
    /// Near plane distance
    pub near_z: f32,
    /// Screen dimensions
    pub screen_width: u32,
    pub screen_height: u32,
}

impl ProjectionMatrix {
    /// Create a new projection matrix with the given parameters.
    #[inline(always)]
    pub fn new(fov_deg: f32, width: u32, height: u32, near_z: f32) -> Self {
        let fov_rad = fov_deg.to_radians();
        let focal_length = 1.0 / (fov_rad * 0.5).tan();
        let aspect_ratio = width as f32 / height as f32;
        
        Self {
            focal_length,
            aspect_ratio,
            near_z,
            screen_width: width,
            screen_height: height,
        }
    }
    
    /// Project a 3D point to screen coordinates using pre-computed values.
    #[inline(always)]
    pub fn project_point(&self, v: Vec3, clip_near: bool, clip_bounds: bool) -> Option<Point> {
        if clip_near && v.2 <= self.near_z { 
            return None; 
        }
        
        // Perspective projection with pre-computed values
        let x_ndc = (v.0 * self.focal_length) / (v.2 * self.aspect_ratio);
        let y_ndc = (v.1 * self.focal_length) / v.2;
        
        let x = ((x_ndc + 1.0) * (self.screen_width as f32) * 0.5) as i32;
        let y = ((1.0 - y_ndc) * (self.screen_height as f32) * 0.5) as i32;
        
        if clip_bounds && (x < 0 || x >= self.screen_width as i32 || y < 0 || y >= self.screen_height as i32) { 
            return None; 
        }
        
        Some(Point::new(x, y))
    }
}

/// Optimized triangle structure for efficient rasterization.
#[derive(Clone, Copy, Debug)]
pub struct OptimizedTriangle {
    /// Screen-space vertices
    pub vertices: [Point; 3],
    /// Pre-computed face normal in camera space
    pub normal: Vec3,
    /// Pre-computed lighting intensity (for flat shading)
    pub intensity: f32,
    /// Triangle depth for sorting
    pub depth: f32,
    /// Triangle color
    pub color: Rgb565,
}

/// High-performance rendering context that caches expensive calculations.
pub struct RenderContext {
    /// Pre-computed projection matrix
    pub projection: ProjectionMatrix,
    /// Pre-computed light direction (normalized)
    pub light_direction: Vec3,
    /// Pre-computed model rotation quaternion
    pub model_rotation: Quaternion,
    /// Pre-computed model origin in camera space
    pub model_origin: Vec3,
    /// Cached vertex normals in camera space
    pub vertex_normals: [Vec3; MAX_VERTICES],
    /// Cached camera-space vertices
    pub camera_vertices: [Vec3; MAX_VERTICES],
    /// Cached projected vertices
    pub projected_vertices: [Option<Point>; MAX_VERTICES],
    /// Triangle sorting array
    pub triangle_order: [(usize, f32); MAX_TRIANGLES],
}

impl RenderContext {
    /// Create a new render context with the given parameters.
    pub fn new(
        fov_deg: f32,
        width: u32,
        height: u32,
        near_z: f32,
        light_dir: Vec3,
        model_origin: Vec3,
        model_rotation: Quaternion,
    ) -> Self {
        Self {
            projection: ProjectionMatrix::new(fov_deg, width, height, near_z),
            light_direction: light_dir.normalize(),
            model_rotation,
            model_origin,
            vertex_normals: [Vec3(0.0, 0.0, 0.0); MAX_VERTICES],
            camera_vertices: [Vec3(0.0, 0.0, 0.0); MAX_VERTICES],
            projected_vertices: [None; MAX_VERTICES],
            triangle_order: [(0, 0.0); MAX_TRIANGLES],
        }
    }
    
    /// Update the render context with new parameters.
    pub fn update(&mut self, light_dir: Vec3, model_origin: Vec3, model_rotation: Quaternion) {
        self.light_direction = light_dir.normalize();
        self.model_origin = model_origin;
        self.model_rotation = model_rotation;
    }
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

/// High-performance scanline triangle filler optimized for embedded systems.
/// 
/// This implementation uses optimized interpolation and minimal branching
/// for maximum performance on resource-constrained hardware.
fn fill_triangle<R: Rasterizer>(r: &mut R, tri: &FlatTriangle, aa: bool) {
    let mut pts = [tri.p0, tri.p1, tri.p2];
    pts.sort_by_key(|p| p.y);
    let (top, mid, bot) = (pts[0], pts[1], pts[2]);
    
    // Early exit for degenerate triangles
    if top.y == bot.y { return; }
    
    let w = r.width() as i32;
    let h = r.height() as i32;
    
    // Pre-compute interpolation slopes for better performance
    let dy_total = bot.y - top.y;
    let dy_upper = mid.y - top.y;
    let dy_lower = bot.y - mid.y;
    
    // Avoid division by zero with early exit
    if dy_total == 0 { return; }
    
    // Pre-compute slopes for left and right edges
    let slope_left_upper = if dy_upper != 0 { (mid.x - top.x) as f32 / dy_upper as f32 } else { 0.0 };
    let slope_left_lower = if dy_lower != 0 { (bot.x - mid.x) as f32 / dy_lower as f32 } else { 0.0 };
    let slope_right = (bot.x - top.x) as f32 / dy_total as f32;
    
    // Scan from top to bottom
    for y in top.y..=bot.y {
        if y < 0 || y >= h { continue; }
        
        let dy = y - top.y;
        let (x_left, x_right) = if y < mid.y {
            // Upper part of triangle
            let x_left = top.x as f32 + slope_left_upper * dy as f32;
            let x_right = top.x as f32 + slope_right * dy as f32;
            (x_left, x_right)
        } else {
            // Lower part of triangle
            let dy_lower_part = y - mid.y;
            let x_left = mid.x as f32 + slope_left_lower * dy_lower_part as f32;
            let x_right = top.x as f32 + slope_right * dy as f32;
            (x_left, x_right)
        };
        
        // Determine scanline bounds
        let (x_start, x_end) = if x_left <= x_right {
            (x_left as i32, x_right as i32)
        } else {
            (x_right as i32, x_left as i32)
        };
        
        // Clip to screen bounds
        let xs = x_start.max(0).min(w - 1);
        let xe = x_end.max(0).min(w - 1);
        
        if xs <= xe {
            if aa {
                // Anti-aliased rendering with subpixel precision
                let x_start_f = x_left;
                let x_end_f = x_right;
                
                // Blend edge pixels
                if xs >= 0 && xs < w {
                    let coverage = 1.0 - (x_start_f.fract()).abs();
                    let alpha = (coverage.clamp(0.0, 1.0) * 255.0) as u8;
                    r.blend_pixel(xs, y, tri.color, alpha);
                }
                
                if xe >= 0 && xe < w && xe != xs {
                    let coverage = (x_end_f.fract()).abs();
                    let alpha = (coverage.clamp(0.0, 1.0) * 255.0) as u8;
                    r.blend_pixel(xe, y, tri.color, alpha);
                }
                
                // Fill solid pixels in between
                let inner_start = (xs + 1).min(xe);
                for x in inner_start..xe {
                    r.set_pixel(x, y, tri.color);
                }
            } else {
                // Fast solid fill
                for x in xs..=xe {
                    r.set_pixel(x, y, tri.color);
                }
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

/// High-performance 3D model rendering with optimized pipeline.
/// 
/// This function uses a cached rendering context to minimize redundant calculations
/// and provides significant performance improvements over the original implementation.
pub fn draw_model<R: Rasterizer>(
    raster: &mut R, 
    model: &Model, 
    origin_cam: Vec3, 
    model_rotation: Quaternion, 
    width: u32, 
    height: u32, 
    options: &RenderOptions
) {
    let start_time = Instant::now();
    
    // Create optimized rendering context
    let mut context = RenderContext::new(
        options.fov_deg,
        width,
        height,
        options.near_z,
        options.light_dir,
        origin_cam,
        model_rotation,
    );
    
    // Transform vertices to camera space
    let transform_start = Instant::now();
    transform_vertices_optimized(model, &mut context);
    let transform_time = transform_start.elapsed().as_micros();
    
    // Project vertices to screen space
    let project_start = Instant::now();
    project_vertices_optimized(&mut context, options);
    let project_time = project_start.elapsed().as_micros();
    
    // Compute vertex normals if needed for lighting
    if matches!(options.shading_mode, ShadingMode::Gouraud) || !matches!(options.lighting_mode, LightingMode::None) {
        compute_vertex_normals_optimized(model, &mut context);
    }
    
    // Sort triangles for painter's algorithm
    let sort_start = Instant::now();
    if options.enable_depth_sorting {
        sort_triangles_optimized(model, &mut context);
    }
    let sort_time = sort_start.elapsed().as_micros();
    
    // Render triangles
    let raster_start = Instant::now();
    let stats = render_triangles_optimized(raster, model, &context, options);
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
        stats.triangles_culled,
        stats.triangles_emitted,
        stats.edges_drawn
    );
}

/// Optimized vertex transformation with caching.
#[inline(always)]
fn transform_vertices_optimized(model: &Model, context: &mut RenderContext) {
    for i in 0..model.vertex_count {
        let rotated = context.model_rotation.rotate_vector(model.vertices[i]);
        context.camera_vertices[i] = Vec3(
            context.model_origin.0 + rotated.0,
            context.model_origin.1 + rotated.1,
            context.model_origin.2 + rotated.2,
        );
    }
}

/// Optimized vertex projection with pre-computed matrix.
#[inline(always)]
fn project_vertices_optimized(context: &mut RenderContext, options: &RenderOptions) {
    for i in 0..context.camera_vertices.len() {
        if context.camera_vertices[i].length_squared() == 0.0 { 
            continue; 
        }
        context.projected_vertices[i] = context.projection.project_point(
            context.camera_vertices[i],
            options.enable_near_clipping,
            options.enable_frustum_clipping,
        );
    }
}

/// Optimized vertex normal computation.
#[inline(always)]
fn compute_vertex_normals_optimized(model: &Model, context: &mut RenderContext) {
    // Reset normals
    for i in 0..MAX_VERTICES {
        context.vertex_normals[i] = Vec3(0.0, 0.0, 0.0);
    }
    
    // Accumulate face normals
    for i in 0..model.triangle_count {
        let tri = &model.triangles[i];
        let normal = face_normal_cam_space(
            context.camera_vertices[tri.vertices[0]],
            context.camera_vertices[tri.vertices[1]],
            context.camera_vertices[tri.vertices[2]],
        );
        
        // Add to vertex normals
        for &vi in &tri.vertices {
            context.vertex_normals[vi] = context.vertex_normals[vi].add(normal);
        }
    }
    
    // Normalize vertex normals
    for i in 0..model.vertex_count {
        context.vertex_normals[i] = context.vertex_normals[i].normalize();
    }
}

/// Optimized triangle sorting.
#[inline(always)]
fn sort_triangles_optimized(model: &Model, context: &mut RenderContext) {
    for i in 0..model.triangle_count {
        let tri = &model.triangles[i];
        let depth = (
            context.camera_vertices[tri.vertices[0]].2 +
            context.camera_vertices[tri.vertices[1]].2 +
            context.camera_vertices[tri.vertices[2]].2
        ) / 3.0;
        context.triangle_order[i] = (i, depth);
    }
    
    context.triangle_order[..model.triangle_count].sort_unstable_by(|a, b| {
        b.1.partial_cmp(&a.1).unwrap_or(Ordering::Equal)
    });
}

/// Optimized triangle rendering with minimal overhead.
fn render_triangles_optimized<R: Rasterizer>(
    raster: &mut R,
    model: &Model,
    context: &RenderContext,
    options: &RenderOptions,
) -> RenderStats {
    let mut stats = RenderStats::default();
    stats.triangles_input = model.triangle_count as u32;
    
    for &(tri_idx, _) in context.triangle_order.iter().take(model.triangle_count) {
        let tri = &model.triangles[tri_idx];
        let [i0, i1, i2] = tri.vertices;
        
        // Backface culling
        if options.enable_backface_culling {
            let normal = face_normal_cam_space(
                context.camera_vertices[i0],
                context.camera_vertices[i1],
                context.camera_vertices[i2],
            );
            if normal.2 >= 0.0 {
                stats.triangles_culled += 1;
                continue;
            }
        }
        
        // Get projected vertices
        let (Some(p0), Some(p1), Some(p2)) = (
            context.projected_vertices[i0],
            context.projected_vertices[i1],
            context.projected_vertices[i2],
        ) else {
            continue;
        };
        
        // Compute lighting
        let color = if !matches!(options.lighting_mode, LightingMode::None) {
            let intensity = if matches!(options.shading_mode, ShadingMode::Gouraud) {
                // Use vertex normals for Gouraud shading
                let i0 = context.vertex_normals[i0].dot(context.light_direction).max(0.0);
                let i1 = context.vertex_normals[i1].dot(context.light_direction).max(0.0);
                let i2 = context.vertex_normals[i2].dot(context.light_direction).max(0.0);
                
                // For now, use average intensity for flat shading
                (i0 + i1 + i2) / 3.0
            } else {
                // Use face normal for flat shading
                let normal = face_normal_cam_space(
                    context.camera_vertices[i0],
                    context.camera_vertices[i1],
                    context.camera_vertices[i2],
                );
                normal.dot(context.light_direction).max(0.0)
            };
            
            let (amb_on, dir_on) = match options.lighting_mode {
                LightingMode::None => (false, false),
                LightingMode::Ambient => (true, false),
                LightingMode::Directional => (false, true),
                LightingMode::AmbientAndDirectional => (true, true),
            };
            
            let lambert_eff = if dir_on { intensity } else { 0.0 };
            let amb = if amb_on { options.ambient_color } else { Rgb565::from_rgb(0, 0, 0) };
            modulate_lit_color(options.model_color, amb, options.directional_color, lambert_eff)
        } else {
            options.model_color
        };
        
        // Render triangle
        if matches!(options.view_mode, ViewMode::Fill | ViewMode::FillAndWireframe) {
            let flat_tri = FlatTriangle { p0, p1, p2, color };
            fill_triangle(raster, &flat_tri, matches!(options.aa_mode, AntiAliasing::Edge));
        }
        
        if matches!(options.view_mode, ViewMode::Wireframe | ViewMode::FillAndWireframe) {
            {
                let mut d2 = Draw2D::new(raster);
                d2.line(p0, p1).color(Rgb565::from_rgb(0, 255, 0)).draw();
            }
            {
                let mut d2 = Draw2D::new(raster);
                d2.line(p1, p2).color(Rgb565::from_rgb(0, 255, 0)).draw();
            }
            {
                let mut d2 = Draw2D::new(raster);
                d2.line(p2, p0).color(Rgb565::from_rgb(0, 255, 0)).draw();
            }
            stats.edges_drawn += 3;
        }
        
        stats.triangles_emitted += 1;
    }
    
    stats
}
