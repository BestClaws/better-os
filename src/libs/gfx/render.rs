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
use defmt::{info, debug, warn, error};
use crate::libs::gfx::Model;
use crate::system::kernel::config::resources::{MAX_TRIANGLES, MAX_VERTICES};
use super::math::{Quaternion, Vec3};

/// Configuration structure for rendering the 3D model.
/// Allows fine-grained control over rendering features for performance vs quality trade-offs.
#[derive(Clone, Copy)]
pub struct RenderOptions {
    /// Field of view in degrees
    pub fov_deg: f32,
    /// Normalized directional light vector
    pub light_dir: Vec3,
    /// Tuple of (min, max) intensity range for grayscale mapping
    pub intensity_range: (f32, f32),
    /// Enable back-face culling to skip invisible triangles
    pub enable_backface_culling: bool,
    /// Enable Z-buffer (not implemented yet)
    pub enable_zbuffer: bool,
    /// Enable lighting calculations
    pub enable_lighting: bool,
    /// Enable triangle depth sorting (Painter's Algorithm)
    pub enable_depth_sorting: bool,
    /// Enable near plane clipping
    pub enable_near_clipping: bool,
    /// Enable frustum bounds clipping
    pub enable_frustum_clipping: bool,
    /// Enable wireframe overlay rendering
    pub enable_wireframe: bool,
    /// Enable filled triangle shading
    pub enable_shading: bool,
    /// Enable anti-aliasing using pixel coverage estimation
    pub enable_antialiasing: bool,
    /// Supersampling factor for anti-aliasing (1 = none, 2 = 2x2, etc.)
    pub antialiasing_factor: u8,
    /// Use edge-only anti-aliasing instead of full supersampling
    pub edge_only_antialiasing: bool,
}

/// Converts a normalized intensity value to a Gray4 color.
#[inline(always)]
fn get_grayscale_color(intensity: f32) -> Gray4 {
    debug!("Converting intensity {} to grayscale", intensity);
    let clamped = intensity.clamp(0.0, 1.0);
    let value = (clamped * 15.0).round() as u8;
    let color = Gray4::new(value);
    debug!("Resulting grayscale color: {:?}", color);
    color
}

/// Projects a 3D vertex to 2D screen coordinates using perspective projection.
#[inline(always)]
fn project(
    v: Vec3,
    fov_deg: f32,
    width: u32,
    height: u32,
    clip_near: bool,
    clip_bounds: bool,
) -> Option<Point> {
    debug!("Projecting vertex {:?}", v);

    // Check near plane clipping
    if clip_near && v.2 <= 0.5 {
        warn!("Vertex clipped at near plane: z={}", v.2);
        return None;
    }

    // Perform perspective projection
    let fov_rad = fov_deg.to_radians();
    let f = 1.0 / (fov_rad / 2.0).tan();
    let aspect = width as f32 / height as f32;

    let x_proj = (v.0 * f) / v.2;
    let y_proj = (v.1 * f) / v.2;

    let x = ((x_proj / aspect + 1.0) * width as f32 * 0.5) as i32;
    let y = ((1.0 - y_proj) * height as f32 * 0.5) as i32;

    // Check frustum bounds
    if clip_bounds && (x < 0 || x >= width as i32 || y < 0 || y >= height as i32) {
        warn!("Vertex clipped outside frustum: x={}, y={}", x, y);
        return None;
    }

    let point = Point::new(x, y);
    debug!("Projected to screen coordinates: {:?}", point);
    Some(point)
}

/// Represents a shaded triangle for rendering with a uniform color.
struct ShadedTriangle {
    p0: Point,
    p1: Point,
    p2: Point,
    color: Gray4,
}

impl OriginDimensions for ShadedTriangle {
    fn size(&self) -> Size {
        debug!("Calculating bounding box for triangle ({:?}, {:?}, {:?})", self.p0, self.p1, self.p2);
        let size = Triangle::new(self.p0, self.p1, self.p2).bounding_box().size;
        debug!("Triangle bounding box size: {:?}", size);
        size
    }
}

impl Drawable for ShadedTriangle {
    type Color = Gray4;
    type Output = ();

    /// Draws a filled triangle using scanline rendering.
    fn draw<D: DrawTarget<Color = Gray4>>(&self, target: &mut D) -> Result<(), D::Error> {
        debug!("Drawing shaded triangle with points ({:?}, {:?}, {:?}) and color {:?}", 
              self.p0, self.p1, self.p2, self.color);

        let mut pts = [self.p0, self.p1, self.p2];
        pts.sort_by_key(|p| p.y);
        let (top, mid, bot) = (pts[0], pts[1], pts[2]);

        if top.y == bot.y {
            debug!("Degenerate triangle detected, skipping");
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
                debug!("Scanline y={} clipped outside display", y);
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
            debug!("Scanline y={}, x range: {} to {}", y, x_start, x_end);

            for x in x_start..=x_end {
                if x >= 0 && x < w {
                    let mut final_color = self.color;

                    if let Some(antialiased_color) = apply_antialiasing(x, y, self) {
                        final_color = antialiased_color;
                        debug!("Applied antialiasing at ({}, {}): {:?}", x, y, final_color);
                    }

                    target.draw_iter(core::iter::once(Pixel(Point::new(x, y), final_color)))?;
                }
            }
        }
        Ok(())
    }
}

/// Applies simple anti-aliasing by fading colors near triangle edges.
fn apply_antialiasing(x: i32, y: i32, tri: &ShadedTriangle) -> Option<Gray4> {
    debug!("Checking antialiasing for pixel ({}, {})", x, y);

    let dist0 = (tri.p0.x - x).abs() + (tri.p0.y - y).abs();
    let dist1 = (tri.p1.x - x).abs() + (tri.p1.y - y).abs();
    let dist2 = (tri.p2.x - x).abs() + (tri.p2.y - y).abs();

    let min_dist = dist0.min(dist1).min(dist2);
    if min_dist < 2 {
        let mut val = tri.color.luma();
        if val > 0 {
            val -= 1;
        }
        let aa_color = Gray4::new(val);
        debug!("Applied antialiasing, color adjusted to {:?}", aa_color);
        return Some(aa_color);
    }
    debug!("No antialiasing applied for pixel ({}, {})", x, y);
    None
}

/// Transforms model vertices to world space and projects them to screen space.
fn transform_and_project_vertices(
    model: &Model,
    origin: Vec3,
    rotation: Quaternion,
    width: u32,
    height: u32,
    options: &RenderOptions,
) -> ([Vec3; MAX_VERTICES], [Option<Point>; MAX_VERTICES]) {
    debug!("Transforming and projecting {} vertices", model.vertex_count);

    let mut world_vertices = [Vec3(0.0, 0.0, 0.0); MAX_VERTICES];
    let mut projected = [None; MAX_VERTICES];

    for i in 0..model.vertex_count {
        debug!("Processing vertex {}: {:?}", i, model.vertices[i]);
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

    (world_vertices, projected)
}

/// Sorts triangles by depth for correct rendering order.
fn sort_triangles(model: &Model, world_vertices: &[Vec3; MAX_VERTICES]) -> [(usize, f32); MAX_TRIANGLES] {
    debug!("Sorting {} triangles by depth", model.triangle_count);

    let mut triangle_meta = [(0usize, 0.0f32); MAX_TRIANGLES];
    for i in 0..model.triangle_count {
        let tri = &model.triangles[i];
        let z0 = world_vertices[tri.vertices[0]].2;
        let z1 = world_vertices[tri.vertices[1]].2;
        let z2 = world_vertices[tri.vertices[2]].2;
        triangle_meta[i] = (i, (z0 + z1 + z2) / 3.0);
        debug!("Triangle {} average depth: {}", i, triangle_meta[i].1);
    }

    triangle_meta
}

/// Renders a 3D model to the display with the specified transformations and options.
pub fn draw_model<D: DrawTarget<Color = Gray4>>(
    display: &mut D,
    model: &Model,
    origin: Vec3,
    rotation: Quaternion,
    width: u32,
    height: u32,
    options: &RenderOptions,
) -> Result<(), D::Error> {
    debug!("Starting model render with {} vertices and {} triangles", 
          model.vertex_count, model.triangle_count);

    let light_dir = options.light_dir.normalize();
    debug!("Normalized light direction: {:?}", light_dir);

    // Transform and project vertices
    let (world_vertices, projected) = transform_and_project_vertices(model, origin, rotation, width, height, options);

    // Sort triangles by depth
    let mut triangle_meta = sort_triangles(model, &world_vertices);
    if options.enable_depth_sorting {
        triangle_meta[..model.triangle_count].sort_unstable_by(|a, b| {
            b.1.partial_cmp(&a.1).unwrap_or(core::cmp::Ordering::Equal)
        });
        debug!("Triangles sorted by depth");
    }

    // Render each triangle
    for &(tri_idx, _) in triangle_meta.iter().take(model.triangle_count) {
        let tri = &model.triangles[tri_idx];
        debug!("Processing triangle {} with vertices {:?}", tri_idx, tri.vertices);

        if let (Some(p0), Some(p1), Some(p2)) = (
            projected[tri.vertices[0]],
            projected[tri.vertices[1]],
            projected[tri.vertices[2]],
        ) {
            // Back-face culling
            if options.enable_backface_culling {
                let edge1 = world_vertices[tri.vertices[1]].sub(world_vertices[tri.vertices[0]]);
                let edge2 = world_vertices[tri.vertices[2]].sub(world_vertices[tri.vertices[0]]);
                let normal = Vec3(
                    edge1.1 * edge2.2 - edge1.2 * edge2.1,
                    edge1.2 * edge2.0 - edge1.0 * edge2.2,
                    edge1.0 * edge2.1 - edge1.1 * edge2.0,
                ).normalize();
                let view_dir = Vec3(0.0, 0.0, -1.0);
                let dot = normal.dot(view_dir);
                if dot <= 0.0 {
                    debug!("Triangle {} culled (back-facing, dot={})", tri_idx, dot);
                    continue;
                }
            }

            // Calculate lighting
            let diffuse = if options.enable_lighting {
                let rotated_normal = rotation.rotate_vector(tri.normal);
                let dot = rotated_normal.dot(light_dir).max(0.0);
                debug!("Triangle {} diffuse lighting: {}", tri_idx, dot);
                dot
            } else {
                1.0
            };

            let intensity = options.intensity_range.0
                + (options.intensity_range.1 - options.intensity_range.0) * diffuse;
            let color = get_grayscale_color(intensity);

            // Draw filled triangle
            if options.enable_shading {
                debug!("Drawing filled triangle {} with color {:?}", tri_idx, color);
                ShadedTriangle { p0, p1, p2, color }.draw(display)?;
            }

            // Draw wireframe
            if options.enable_wireframe {
                debug!("Drawing wireframe for triangle {}", tri_idx);
                let style = PrimitiveStyle::with_stroke(Gray4::new(15), 1);
                Line::new(p0, p1).into_styled(style).draw(display)?;
                Line::new(p1, p2).into_styled(style).draw(display)?;
                Line::new(p2, p0).into_styled(style).draw(display)?;
            }
        } else {
            warn!("Triangle {} skipped due to projection failure", tri_idx);
        }
    }

    debug!("Model rendering completed");
    Ok(())
}