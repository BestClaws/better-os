use core::iter::Iterator;
use defmt::info;
use embassy_time::{Duration, Instant, Timer};
use embedded_graphics::{
    pixelcolor::BinaryColor,
    prelude::*,
    primitives::Triangle,
    geometry::{Point, Size},
    Drawable,
    text::{Text, Alignment, Baseline},
    mono_font::{ MonoTextStyle},
};
use embedded_graphics::mono_font::ascii::FONT_6X10;
use micromath::F32Ext;
use crate::system::app::app_context::AppContext;
use crate::util::math::primitives::Vec3;

// Quaternion struct for rotation
#[derive(Clone, Copy)]
struct Quaternion {
    w: f32,
    x: f32,
    y: f32,
    z: f32,
}

impl Quaternion {
    fn from_axis_angle(axis: Vec3, angle_rad: f32) -> Self {
        let (sin_a, cos_a) = (angle_rad / 2.0).sin_cos();
        let mag = (axis.0 * axis.0 + axis.1 * axis.1 + axis.2 * axis.2).sqrt();
        let (x, y, z) = if mag < 0.0001 {
            (0.0, 0.0, 0.0)
        } else {
            (axis.0 / mag * sin_a, axis.1 / mag * sin_a, axis.2 / mag * sin_a)
        };
        Quaternion { w: cos_a, x, y, z }
    }

    fn mul(self, other: Quaternion) -> Quaternion {
        Quaternion {
            w: self.w * other.w - self.x * other.x - self.y * other.y - self.z * other.z,
            x: self.w * other.x + self.x * other.w + self.y * other.z - self.z * other.y,
            y: self.w * other.y - self.x * other.z + self.y * other.w + self.z * other.x,
            z: self.w * other.z + self.x * other.y - self.y * other.x + self.z * other.w,
        }
    }

    fn rotate_vector(self, v: Vec3) -> Vec3 {
        let q_vec = Quaternion { w: 0.0, x: v.0, y: v.1, z: v.2 };
        let q_conj = Quaternion { w: self.w, x: -self.x, y: -self.y, z: -self.z };
        let result = self.mul(q_vec).mul(q_conj);
        Vec3(result.x, result.y, result.z)
    }
}

// Projects a 3D point to 2D screen coordinates, adjusted for cubic appearance
fn project(v: Vec3, fov_deg: f32, width: u32, height: u32) -> Option<Point> {
    if v.2 <= 0.5 {
        // info!("Vertex clipped: z={}", v.2); // Debug: log clipped vertices
        return None;
    }

    let fov_rad = fov_deg.to_radians();
    let f = 1.0 / (fov_rad / 2.0).tan(); // Same focal length for x and y

    let x_proj = (v.0 * f) / v.2;
    let y_proj = (v.1 * f) / v.2;

    // Map to screen, adjusting for 2:1 aspect ratio to keep cube cubic
    let aspect = width as f32 / height as f32; // 128 / 64 = 2.0
    let x = ((x_proj / aspect + 1.0) * (width as f32 / 2.0)) as i32;
    let y = ((1.0 - y_proj) * (height as f32 / 2.0)) as i32;

    // Clip to viewport bounds (128x64)
    if x < 0 || x >= width as i32 || y < 0 || y >= height as i32 {
        // info!("Vertex out of bounds: x={}, y={}", x, y); // Debug: log out-of-bounds
        return None;
    }

    Some(Point::new(x, y))
}

// 2x2 ordered dithering matrix
const DITHER_MATRIX: [[f32; 2]; 2] = [[0.0, 0.5], [0.75, 0.25]];

// Applies dithering with adjusted intensity
fn should_draw_pixel(x: i32, y: i32, intensity: f32) -> bool {
    let matrix_size = 2;
    let dither_value = DITHER_MATRIX[(y as usize % matrix_size)][(x as usize % matrix_size)];
    intensity > dither_value
}

// Draws a cube with dithered faces and depth sorting
fn draw_cube<D: DrawTarget<Color = BinaryColor>>(
    display: &mut D,
    origin: Vec3,
    size: f32,
    fov_deg: f32,
    width: u32,
    height: u32,
    rotation: Quaternion,
    light_dir: Vec3,
) -> Result<(), D::Error> {
    // Define cube vertices (centered at origin, side length = size)
    let s = size / 2.0;
    let vertices = [
        Vec3(-s, -s, -s), // 0: Back bottom left
        Vec3(s, -s, -s),  // 1: Back bottom right
        Vec3(s, s, -s),   // 2: Back top right
        Vec3(-s, s, -s),  // 3: Back top left
        Vec3(-s, -s, s),  // 4: Front bottom left
        Vec3(s, -s, s),   // 5: Front bottom right
        Vec3(s, s, s),    // 6: Front top right
        Vec3(-s, s, s),   // 7: Front top left
    ];

    // Define faces with normals (two triangles per face)
    let faces = [
        ([0, 1, 2], Vec3(0.0, 0.0, -1.0)), // Back face 1
        ([2, 3, 0], Vec3(0.0, 0.0, -1.0)), // Back face 2
        ([4, 5, 6], Vec3(0.0, 0.0, 1.0)),  // Front face 1
        ([6, 7, 4], Vec3(0.0, 0.0, 1.0)),  // Front face 2
        ([0, 1, 5], Vec3(0.0, -1.0, 0.0)), // Bottom face 1
        ([5, 4, 0], Vec3(0.0, -1.0, 0.0)), // Bottom face 2
        ([2, 3, 7], Vec3(0.0, 1.0, 0.0)),  // Top face 1
        ([7, 6, 2], Vec3(0.0, 1.0, 0.0)),  // Top face 2
        ([0, 3, 7], Vec3(-1.0, 0.0, 0.0)), // Left face 1
        ([7, 4, 0], Vec3(-1.0, 0.0, 0.0)), // Left face 2
        ([1, 2, 6], Vec3(1.0, 0.0, 0.0)),  // Right face 1
        ([6, 5, 1], Vec3(1.0, 0.0, 0.0)),  // Right face 2
    ];

    // Normalize light direction
    let light_dir = {
        let mag = (light_dir.0 * light_dir.0 + light_dir.1 * light_dir.1 + light_dir.2 * light_dir.2).sqrt();
        if mag < 0.0001 {
            Vec3(0.0, 0.0, -1.0)
        } else {
            Vec3(light_dir.0 / mag, light_dir.1 / mag, light_dir.2 / mag)
        }
    };

    // Transform and project vertices
    let mut projected = [None; 8];
    let mut world_vertices = [Vec3(0.0, 0.0, 0.0); 8];
    for (i, v) in vertices.iter().enumerate() {
        let rotated = rotation.rotate_vector(*v);
        let world = Vec3(origin.0 + rotated.0, origin.1 + rotated.1, origin.2 + rotated.2);
        world_vertices[i] = world;
        projected[i] = project(world, fov_deg, width, height);
        // info!("Vertex {}: {:?}", i, world); // Debug: log vertex positions
    }

    // Collect triangles with their average z for depth sorting
    let mut triangles = [(0, 0.0, Vec3(0.0, 0.0, 0.0)); 12];
    for (i, (indices, normal)) in faces.iter().enumerate() {
        let z_avg = (world_vertices[indices[0]].2 +
            world_vertices[indices[1]].2 +
            world_vertices[indices[2]].2) / 3.0;
        triangles[i] = (i, z_avg, *normal);
    }

    // Sort triangles by z_avg (descending, farthest first)
    triangles.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(core::cmp::Ordering::Equal));

    // Custom drawable for dithered triangle with scanline rasterization
    struct DitheredTriangle {
        p0: Point,
        p1: Point,
        p2: Point,
        intensity: f32,
    }

    impl OriginDimensions for DitheredTriangle {
        fn size(&self) -> Size {
            Triangle::new(self.p0, self.p1, self.p2).bounding_box().size
        }
    }

    impl Drawable for DitheredTriangle {
        type Color = BinaryColor;
        type Output = ();

        fn draw<D: DrawTarget<Color = BinaryColor>>(&self, target: &mut D) -> Result<Self::Output, D::Error> {
            // Sort vertices by y-coordinate (top to bottom)
            let mut points = [self.p0, self.p1, self.p2];
            points.sort_by_key(|p| p.y);
            let (top, mid, bot) = (points[0], points[1], points[2]);

            // Handle degenerate triangle
            if top.y == bot.y || top.x == mid.x && mid.x == bot.x {
                return Ok(());
            }

            // Get canvas dimensions
            let canvas_width = target.bounding_box().size.width as i32;
            let canvas_height = target.bounding_box().size.height as i32;

            // Compute slopes and x-ranges for scanlines
            let x0 = top.x as f32;
            let x1 = mid.x as f32;
            let x2 = bot.x as f32;
            let y0 = top.y as f32;
            let y1 = mid.y as f32;
            let y2 = bot.y as f32;

            let slope_top_bot = if y2 != y0 { (x2 - x0) / (y2 - y0) } else { 0.0 };
            let slope_top_mid = if y1 != y0 { (x1 - x0) / (y1 - y0) } else { 0.0 };
            let slope_mid_bot = if y2 != y1 { (x2 - x1) / (y2 - y1) } else { 0.0 };

            // Draw top half (top to mid)
            let pixels_top_half = (top.y..mid.y).flat_map(|y| {
                let x_start = (x0 + slope_top_bot * (y as f32 - y0)).round() as i32;
                let x_end = (x0 + slope_top_mid * (y as f32 - y0)).round() as i32;
                let (x_min, x_max) = if x_start <= x_end { (x_start, x_end + 1) } else { (x_end, x_start + 1) };
                let x_min = x_min.max(0).min(canvas_width);
                let x_max = x_max.max(0).min(canvas_width);
                (x_min..x_max).filter_map(move |x| {
                    if y >= 0 && y < canvas_height && should_draw_pixel(x, y, self.intensity) {
                        Some(Pixel(Point::new(x, y), BinaryColor::On))
                    } else {
                        None
                    }
                })
            });

            // Draw bottom half (mid to bot)
            let pixels_bottom_half = (mid.y..=bot.y).flat_map(|y| {
                let x_start = (x0 + slope_top_bot * (y as f32 - y0)).round() as i32;
                let x_end = (x1 + slope_mid_bot * (y as f32 - y1)).round() as i32;
                let (x_min, x_max) = if x_start <= x_end { (x_start, x_end + 1) } else { (x_end, x_start + 1) };
                let x_min = x_min.max(0).min(canvas_width);
                let x_max = x_max.max(0).min(canvas_width);
                (x_min..x_max).filter_map(move |x| {
                    if y >= 0 && y < canvas_height && should_draw_pixel(x, y, self.intensity) {
                        Some(Pixel(Point::new(x, y), BinaryColor::On))
                    } else {
                        None
                    }
                })
            });

            // Combine iterators and draw
            target.draw_iter(pixels_top_half.chain(pixels_bottom_half))?;
            Ok(())
        }
    }

    // Draw triangles in depth-sorted order
    for (face_idx, _z_avg, normal) in triangles.iter() {
        let (indices, _) = faces[*face_idx];
        if let (Some(p0), Some(p1), Some(p2)) = (
            projected[indices[0]],
            projected[indices[1]],
            projected[indices[2]],
        ) {
            // Rotate normal for lighting
            let rotated_normal = rotation.rotate_vector(*normal);

            // Compute diffuse lighting, scaled to [0.2, 0.8]
            let raw_intensity = (rotated_normal.0 * light_dir.0 +
                rotated_normal.1 * light_dir.1 +
                rotated_normal.2 * light_dir.2)
                .max(0.0)
                .clamp(0.0, 1.0);
            let intensity = 0.2 + 0.6 * raw_intensity; // Map [0, 1] to [0.2, 0.8]

            // Draw dithered triangle
            DitheredTriangle { p0, p1, p2, intensity }.draw(display)?;
        }
    }

    Ok(())
}

#[embassy_executor::task]
pub async fn cube_app(context: AppContext) {
    // Initialize quaternion (identity)
    let mut rotation = Quaternion { w: 1.0, x: 0.0, y: 0.0, z: 0.0 };
    // Rotation speed: 1 degree per frame (~60 deg/s at 60 FPS)
    let angle_increment = 2.0f32.to_radians();
    // For FPS calculation
    let mut last_time = Instant::now();
    let mut frame_count = 0u32;
    let mut fps = 0.0f32;

    loop {
        if !context.is_focused().await {
            Timer::after(Duration::from_millis(100)).await;
            continue;
        }

        // Apply rotations around X (roll), Y (pitch), and Z (yaw)
        let rot_x = Quaternion::from_axis_angle(Vec3(1.0, 0.0, 0.0), angle_increment);
        let rot_y = Quaternion::from_axis_angle(Vec3(0.0, 1.0, 0.0), angle_increment);
        let rot_z = Quaternion::from_axis_angle(Vec3(0.0, 0.0, 1.0), angle_increment);
        rotation = rot_z.mul(rot_y.mul(rot_x.mul(rotation)));

        // Calculate FPS
        frame_count += 1;
        let current_time = Instant::now();
        let elapsed = current_time - last_time;
        if elapsed.as_millis() >= 1000 {
            fps = frame_count as f32 * 1000.0 / elapsed.as_millis() as f32;
            frame_count = 0;
            last_time = current_time;
        }

        context.draw(|canvas| {
            canvas.clear();

            // Draw cube
            let light_dir = Vec3(0.4, -0.4, -0.8);
            if let Err(e) = draw_cube(
                canvas,
                Vec3(0.0, 0.0, 3.0), // Centered at z=3.0
                1.0,                 // Unit cube size
                45.0,                // FOV
                canvas.width(),
                canvas.height(),
                rotation,
                light_dir,
            ) {
                info!("Draw error: {:?}", e);
            }

            use core::fmt::Write;

            let mut text_buf = heapless::String::<32>::new();
            write!(text_buf, "FPS: {:.1}", fps).ok();

            let style = MonoTextStyle::new(&FONT_6X10, BinaryColor::On);
            Text::new(&text_buf, Point::new(0, 10), style)
                .draw(canvas)
                .unwrap();


        }).await;

        context.request_redraw().await;
        Timer::after(Duration::from_millis(16)).await;
    }
}