use core::iter::Iterator;
use embedded_graphics::{
    prelude::{DrawTarget, Point, Size},
    Drawable, Pixel,
    pixelcolor::Gray4,
};
use embedded_graphics::prelude::{Dimensions, OriginDimensions};
use embedded_graphics::primitives::Triangle;
use micromath::F32Ext;

use super::math::{Quaternion, Vec3};
use super::model::{Model, MAX_TRIANGLES, MAX_VERTICES};

/// User-configurable render options.
pub struct RenderOptions {
    pub fov_deg: f32,             // Field of view in degrees
    pub light_dir: Vec3,          // Direction of light source
    pub intensity_range: (f32, f32), // Min and max lighting intensity
}

/// Converts intensity (0.0–1.0) into a 4-bit grayscale color.
fn get_grayscale_color(intensity: f32) -> Gray4 {
    let clamped = intensity.clamp(0.0, 1.0);
    let value = (clamped * 15.0) as u8; // Avoid round() for speed
    Gray4::new(value)
}

/// Perspective projects a 3D point to 2D screen space.
fn project(v: Vec3, fov_rad: f32, f: f32, width: f32, height: f32) -> Option<Point> {
    // Skip points behind or too close to the camera
    if v.2 <= 0.5 {
        return None;
    }

    // Compute perspective projection
    let aspect = width / height;
    let x_proj = (v.0 * f) / v.2 / aspect;
    let y_proj = (v.1 * f) / v.2;

    // Map to screen coordinates
    let x = ((x_proj + 1.0) * 0.5 * width) as i32;
    let y = ((1.0 - y_proj) * 0.5 * height) as i32;

    // Check bounds
    if x < 0 || x >= width as i32 || y < 0 || y >= height as i32 {
        return None;
    }

    Some(Point::new(x, y))
}

/// A triangle filled using scanlines with grayscale shading.
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
        // Step 1: Sort vertices by y-coordinate (top to bottom)
        let mut points = [self.p0, self.p1, self.p2];
        points.sort_by_key(|p| p.y);

        let (top, mid, bot) = (points[0], points[1], points[2]);

        // Step 2: Skip degenerate triangles
        if top.y == bot.y {
            return Ok(());
        }

        // Step 3: Get canvas dimensions
        let canvas_size = target.bounding_box().size;
        let w = canvas_size.width as i32;
        let h = canvas_size.height as i32;

        // Step 4: Edge interpolation function using integer arithmetic
        let edge = |a: Point, b: Point, y: i32| -> i32 {
            if b.y == a.y {
                a.x
            } else {
                // Integer division to reduce floating-point usage
                a.x + ((b.x - a.x) * (y - a.y)) / (b.y - a.y)
            }
        };

        // Step 5: Draw top half (top to mid)
        for y in top.y..mid.y {
            if y < 0 || y >= h {
                continue;
            }
            let x_a = edge(top, bot, y);
            let x_b = edge(top, mid, y);
            let x_start = x_a.min(x_b).clamp(0, w - 1);
            let x_end = x_a.max(x_b).clamp(0, w - 1);
            // Batch pixels for the entire scanline
            let pixels = (x_start..=x_end).map(|x| Pixel(Point::new(x, y), self.color));
            target.draw_iter(pixels)?;
        }

        // Step 6: Draw bottom half (mid to bot)
        for y in mid.y..=bot.y {
            if y < 0 || y >= h {
                continue;
            }
            let x_a = edge(top, bot, y);
            let x_b = edge(mid, bot, y);
            let x_start = x_a.min(x_b).clamp(0, w - 1);
            let x_end = x_a.max(x_b).clamp(0, w - 1);
            // Batch pixels for the entire scanline
            let pixels = (x_start..=x_end).map(|x| Pixel(Point::new(x, y), self.color));
            target.draw_iter(pixels)?;
        }

        Ok(())
    }
}

/// Renders a 3D model to a 2D display using software rasterization.
pub fn draw_model<D: DrawTarget<Color = Gray4>>(
    display: &mut D,
    model: &Model,
    origin: Vec3,
    rotation: Quaternion,
    width: u32,
    height: u32,
    options: &RenderOptions,
) -> Result<(), D::Error> {
    // Step 1: Precompute projection constants
    let fov_rad = options.fov_deg.to_radians();
    let f = 1.0 / (fov_rad / 2.0).tan(); // Focal length
    let width_f = width as f32;
    let height_f = height as f32;

    // Step 2: Normalize light direction once
    let light_dir = options.light_dir.normalize();
    let (min_intensity, max_intensity) = options.intensity_range;
    let intensity_scale = max_intensity - min_intensity;

    // Step 3: Transform and project vertices
    let mut projected = [None; MAX_VERTICES];
    let mut world_vertices = [Vec3(0.0, 0.0, 0.0); MAX_VERTICES];

    for i in 0..model.vertex_count {
        let v = rotation.rotate_vector(model.vertices[i]);
        let world = Vec3(v.0 + origin.0, v.1 + origin.1, v.2 + origin.2);
        world_vertices[i] = world;
        projected[i] = project(world, fov_rad, f, width_f, height_f);
    }

    // Step 4: Prepare triangles for z-sorting and culling
    let mut triangles = [(0, 0.0, Vec3(0.0, 0.0, 0.0)); MAX_TRIANGLES];
    let mut valid_triangle_count = 0;

    for i in 0..model.triangle_count {
        let tri = model.triangles[i];
        let v0_idx = tri.vertices[0];
        let v1_idx = tri.vertices[1];
        let v2_idx = tri.vertices[2];

        // Skip if any vertex is invalid
        if projected[v0_idx].is_none() || projected[v1_idx].is_none() || projected[v2_idx].is_none() {
            continue;
        }

        // Backface culling: skip triangles facing away from camera
        let rotated_normal = rotation.rotate_vector(tri.normal);
        let camera_dir = Vec3(0.0, 0.0, -1.0); // Camera looks along negative z
        if rotated_normal.dot(camera_dir) > 0.0 {
            continue;
        }

        // Compute average z-depth for sorting
        let z_avg = (
            world_vertices[v0_idx].2 +
                world_vertices[v1_idx].2 +
                world_vertices[v2_idx].2
        ) * 0.33333333; // Avoid division for speed

        triangles[valid_triangle_count] = (i, z_avg, rotated_normal);
        valid_triangle_count += 1;
    }

    // Step 5: Sort valid triangles from far to near
    triangles[0..valid_triangle_count].sort_unstable_by(|a, b| {
        b.1.partial_cmp(&a.1).unwrap_or(core::cmp::Ordering::Equal)
    });

    // Step 6: Render valid triangles
    for (i, _, rotated_normal) in triangles.iter().take(valid_triangle_count) {
        let tri = model.triangles[*i];
        let (p0, p1, p2) = (
            projected[tri.vertices[0]].unwrap(),
            projected[tri.vertices[1]].unwrap(),
            projected[tri.vertices[2]].unwrap(),
        );

        // Compute lighting intensity (Lambertian shading)
        let raw_intensity = rotated_normal.dot(light_dir).max(0.0);
        let scaled = min_intensity + intensity_scale * raw_intensity;
        let color = get_grayscale_color(scaled);

        // Draw the triangle
        let shaded_triangle = ShadedTriangle { p0, p1, p2, color };
        shaded_triangle.draw(display)?;
    }

    Ok(())
}