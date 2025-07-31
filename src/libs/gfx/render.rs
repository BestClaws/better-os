#![no_std]

use core::iter::Iterator;
use embedded_graphics::{
    pixelcolor::Gray4,
    prelude::{DrawTarget, Dimensions, OriginDimensions, Point, Size},
    Drawable, Pixel,
};
use embedded_graphics::primitives::Triangle;
use micromath::F32Ext;

use super::math::{Quaternion, Vec3};
use super::model::{Model, MAX_TRIANGLES, MAX_VERTICES};

/// Rendering options for the 3D software renderer
pub struct RenderOptions {
    /// Field of view in degrees
    pub fov_deg: f32,
    /// Direction of the light vector
    pub light_dir: Vec3,
    /// Range of lighting intensity (e.g., from 0.2 to 1.0)
    pub intensity_range: (f32, f32),
}

/// Converts normalized light intensity to a Gray4 color (4-bit grayscale)
fn get_grayscale_color(intensity: f32) -> Gray4 {
    let clamped = intensity.clamp(0.0, 1.0);
    let value = (clamped * 15.0).round() as u8;
    Gray4::new(value)
}

/// Projects a 3D vector to a 2D screen point using perspective projection
fn project(v: Vec3, fov_deg: f32, width: u32, height: u32) -> Option<Point> {
    if v.2 <= 0.5 {
        return None;
    }

    let fov_rad = fov_deg.to_radians();
    let f = 1.0 / (fov_rad / 2.0).tan();
    let aspect = width as f32 / height as f32;

    let x_proj = (v.0 * f) / v.2;
    let y_proj = (v.1 * f) / v.2;

    let x = ((x_proj / aspect + 1.0) * width as f32 * 0.5) as i32;
    let y = ((1.0 - y_proj) * height as f32 * 0.5) as i32;

    if x < 0 || x >= width as i32 || y < 0 || y >= height as i32 {
        return None;
    }

    Some(Point::new(x, y))
}

/// A triangle filled with a single shade of Gray4 color
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
        let mut points = [self.p0, self.p1, self.p2];
        points.sort_by_key(|p| p.y); // Sort top to bottom

        let (top, mid, bot) = (points[0], points[1], points[2]);
        let canvas = target.bounding_box();
        let w = canvas.size.width as i32;
        let h = canvas.size.height as i32;

        if top.y == bot.y {
            return Ok(()); // Degenerate triangle
        }

        let interp = |y: i32, y0: i32, y1: i32, x0: i32, x1: i32| {
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
                    let p = Point::new(x, y);
                    target.draw_iter(core::iter::once(Pixel(p, self.color)))?;
                }
            }
        }

        Ok(())
    }
}

/// Renders a 3D model using flat-shaded triangles onto a 2D embedded-graphics `DrawTarget`.
///
/// This function performs:
/// - Vertex transformation (rotation + translation)
/// - Perspective projection
/// - Z-sorted triangle rasterization with lighting
/// - Flat shading using grayscale
///
/// Requirements:
/// - `MAX_VERTICES` and `MAX_TRIANGLES` must fit the model
/// - No allocation or dynamic memory used
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

    // Step 1: Transform and project vertices
    let mut world_vertices = [Vec3(0.0, 0.0, 0.0); MAX_VERTICES];
    let mut projected = [None; MAX_VERTICES];

    for i in 0..model.vertex_count {
        let rotated = rotation.rotate_vector(model.vertices[i]);
        let world = Vec3(
            origin.0 + rotated.0,
            origin.1 + rotated.1,
            origin.2 + rotated.2,
        );
        world_vertices[i] = world;
        projected[i] = project(world, options.fov_deg, width, height);
    }

    // Step 2: Compute triangle depths for sorting
    let mut triangle_meta = [(0usize, 0.0f32); MAX_TRIANGLES];
    for i in 0..model.triangle_count {
        let tri = &model.triangles[i];
        let z_avg = (world_vertices[tri.vertices[0]].2
            + world_vertices[tri.vertices[1]].2
            + world_vertices[tri.vertices[2]].2)
            / 3.0;
        triangle_meta[i] = (i, z_avg);
    }

    // Step 3: Painter's algorithm - back-to-front rendering
    triangle_meta[0..model.triangle_count].sort_by(|a, b| {
        b.1.partial_cmp(&a.1).unwrap_or(core::cmp::Ordering::Equal)
    });

    for (tri_idx, _) in triangle_meta.iter().take(model.triangle_count) {
        let tri = &model.triangles[*tri_idx];

        // Back-face culling optional (not enabled here)
        let Some(p0) = projected[tri.vertices[0]] else { continue };
        let Some(p1) = projected[tri.vertices[1]] else { continue };
        let Some(p2) = projected[tri.vertices[2]] else { continue };

        // Step 4: Lighting
        let rotated_normal = rotation.rotate_vector(tri.normal);
        let raw = rotated_normal.dot(light_dir).max(0.0);
        let intensity = options.intensity_range.0
            + (options.intensity_range.1 - options.intensity_range.0) * raw;

        let color = get_grayscale_color(intensity);

        // Step 5: Rasterize
        ShadedTriangle {
            p0,
            p1,
            p2,
            color,
        }
            .draw(display)?;
    }

    Ok(())
}
