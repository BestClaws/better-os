use alloc::vec::Vec;
use defmt::info;
use embassy_time::{Duration, Timer};
use embedded_graphics::{
    pixelcolor::BinaryColor,
    prelude::*,
    primitives::{Line, PrimitiveStyle, Triangle},
    geometry::{Point, Size},
    Drawable,
};
use micromath::F32Ext;
use crate::system::app::app_context::AppContext;
use crate::system::services::battery_srv::BATTERY_CHANNEL;
use crate::system::services::gyro_accel_srv::ORIENTATION_CHANNEL;
use crate::system::ui::canvas::Canvas;
use crate::system::vendor::invensense::drivers::mpu6050::sensor::{get_gravity, get_yaw_pitch_roll};
use crate::util::math::primitives::Vec3;

// Projects a 3D point to 2D screen coordinates
fn project(v: Vec3, fov_deg: f32, width: u32, height: u32) -> Option<(i32, i32)> {
    if v.2 <= 0.1 {
        return None;
    }

    let fov_rad = fov_deg.to_radians();
    let aspect = width as f32 / height as f32;
    let f = 1.0 / (fov_rad / 2.0).tan();

    let x_proj = (v.0 * f) / v.2;
    let y_proj = (v.1 * f) / (v.2 * aspect);

    Some((
        ((x_proj + 1.0) * (width as f32 / 2.0)) as i32,
        ((1.0 - y_proj) * (height as f32 / 2.0)) as i32,
    ))
}

// Computes a rotation matrix to align the slab's Z-axis with a target direction
fn align_to_direction(v: Vec3, target: Vec3) -> [[f32; 3]; 3] {
    let v = {
        let mag = (v.0 * v.0 + v.1 * v.1 + v.2 * v.2).sqrt();
        if mag < 0.0001 {
            Vec3(0.0, 0.0, 1.0)
        } else {
            Vec3(v.0 / mag, v.1 / mag, v.2 / mag)
        }
    };
    let target = {
        let mag = (target.0 * target.0 + target.1 * target.1 + target.2 * target.2).sqrt();
        if mag < 0.0001 {
            Vec3(0.0, 0.0, 1.0)
        } else {
            Vec3(target.0 / mag, target.1 / mag, target.2 / mag)
        }
    };

    let axis = Vec3(
        v.1 * target.2 - v.2 * target.1,
        v.2 * target.0 - v.0 * target.2,
        v.0 * target.1 - v.1 * target.0,
    );
    let axis_mag = (axis.0 * axis.0 + axis.1 * axis.1 + axis.2 * axis.2).sqrt();
    let dot = v.0 * target.0 + v.1 * target.1 + v.2 * target.2;
    let angle = dot.clamp(-1.0, 1.0).acos();

    if axis_mag < 0.0001 || angle < 0.0001 {
        return [
            [1.0, 0.0, 0.0],
            [0.0, 1.0, 0.0],
            [0.0, 0.0, 1.0],
        ];
    }

    let axis = Vec3(axis.0 / axis_mag, axis.1 / axis_mag, axis.2 / axis_mag);
    let (s, c) = angle.sin_cos();
    let one_minus_c = 1.0 - c;

    [
        [
            c + axis.0 * axis.0 * one_minus_c,
            axis.0 * axis.1 * one_minus_c - axis.2 * s,
            axis.0 * axis.2 * one_minus_c + axis.1 * s,
        ],
        [
            axis.0 * axis.1 * one_minus_c + axis.2 * s,
            c + axis.1 * axis.1 * one_minus_c,
            axis.1 * axis.2 * one_minus_c - axis.0 * s,
        ],
        [
            axis.0 * axis.2 * one_minus_c - axis.1 * s,
            axis.1 * axis.2 * one_minus_c + axis.0 * s,
            c + axis.2 * axis.2 * one_minus_c,
        ],
    ]
}

// Applies a rotation matrix to a 3D vector
fn apply_rotation(v: Vec3, rotation: &[[f32; 3]; 3]) -> Vec3 {
    Vec3(
        v.0 * rotation[0][0] + v.1 * rotation[0][1] + v.2 * rotation[0][2],
        v.0 * rotation[1][0] + v.1 * rotation[1][1] + v.2 * rotation[1][2],
        v.0 * rotation[2][0] + v.1 * rotation[2][1] + v.2 * rotation[2][2],
    )
}

// Simple 2x2 ordered dithering matrix
const DITHER_MATRIX: [[f32; 2]; 2] = [
    [0.0, 0.5],
    [0.75, 0.25],
];

// Applies dithering to determine if a pixel should be drawn based on intensity
fn should_draw_pixel(x: i32, y: i32, intensity: f32) -> bool {
    let matrix_size = 2;
    let dither_value = DITHER_MATRIX[(y as usize % matrix_size)][(x as usize % matrix_size)];
    intensity > dither_value
}

// Draws a 3D slab with lighting and dithered shading
fn draw_slab<D: DrawTarget<Color = BinaryColor>>(
    display: &mut D,
    origin: Vec3,
    size: f32,
    fov_deg: f32,
    width: u32,
    height: u32,
    direction: Vec3,
    slab_width: f32,
    slab_height: f32,
    slab_length: f32,
    light_dir: Vec3,
) -> Result<(), D::Error> {
    // Define slab dimensions
    let half_width = slab_width / 2.0;
    let half_height = slab_height / 2.0;
    let half_length = slab_length / 2.0;

    // Define slab vertices (local coordinates, long axis along Z)
    let vertices = [
        Vec3(-half_width, -half_height, -half_length), // Back bottom left
        Vec3(half_width, -half_height, -half_length),  // Back bottom right
        Vec3(half_width, half_height, -half_length),   // Back top right
        Vec3(-half_width, half_height, -half_length),  // Back top left
        Vec3(-half_width, -half_height, half_length),  // Front bottom left
        Vec3(half_width, -half_height, half_length),   // Front bottom right
        Vec3(half_width, half_height, half_length),    // Front top right
        Vec3(-half_width, half_height, half_length),   // Front top left
    ];

    // Define edges for wireframe rendering
    let edges = [
        (0, 1), (1, 2), (2, 3), (3, 0), // Back face
        (4, 5), (5, 6), (6, 7), (7, 4), // Front face
        (0, 4), (1, 5), (2, 6), (3, 7), // Connecting edges
    ];

    // Define triangular faces and their normals
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

    // Compute rotation matrix to align slab's Z-axis with direction
    let rotation = align_to_direction(Vec3(0.0, 0.0, 1.0), direction);

    // Transform and project vertices
    let mut projected = [None; 8];
    for (i, &v) in vertices.iter().enumerate() {
        let rotated = apply_rotation(v, &rotation);
        let world = Vec3(
            origin.0 + rotated.0 * size,
            origin.1 + rotated.1 * size,
            origin.2 + rotated.2 * size,
        );
        projected[i] = project(world, fov_deg, width, height).map(Point::from);
    }

    // Normalize light direction
    let light_dir = {
        let mag = (light_dir.0 * light_dir.0 + light_dir.1 * light_dir.1 + light_dir.2 * light_dir.2).sqrt();
        if mag < 0.0001 {
            Vec3(0.0, 0.0, -1.0)
        } else {
            Vec3(light_dir.0 / mag, light_dir.1 / mag, light_dir.2 / mag)
        }
    };

    // Draw filled triangles with dithered shading
    for (indices, normal) in faces.iter() {
        if let (Some(p0), Some(p1), Some(p2)) = (
            projected[indices[0]],
            projected[indices[1]],
            projected[indices[2]],
        ) {
            // Rotate normal
            let rotated_normal = apply_rotation(*normal, &rotation);
            // Compute diffuse lighting (Lambertian)
            let intensity = (rotated_normal.0 * light_dir.0 +
                rotated_normal.1 * light_dir.1 +
                rotated_normal.2 * light_dir.2)
                .max(0.0)
                .clamp(0.0, 1.0);

            // Create a custom drawable for dithered triangle
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
                    let bounds = Triangle::new(self.p0, self.p1, self.p2).bounding_box();
                    let mut pixels = Vec::new();
                    let top_left = bounds.top_left;
                    if let Some(bottom_right) = bounds.bottom_right() {
                        for x in top_left.x..=bottom_right.x {
                            for y in top_left.y..=bottom_right.y {
                                let p = Point::new(x, y);
                                if Triangle::new(self.p0, self.p1, self.p2).contains(p) {
                                    if should_draw_pixel(x, y, self.intensity) {
                                        pixels.push(Pixel(p, BinaryColor::On));
                                    }
                                }
                            }
                        }
                    }
                    target.draw_iter(pixels)?;
                    Ok(())
                }
            }

            DitheredTriangle {
                p0,
                p1,
                p2,
                intensity,
            }
                .draw(display)?;
        }
    }

    // Draw edges for wireframe
    for &(i1, i2) in edges.iter() {
        if let (Some(p1), Some(p2)) = (projected[i1], projected[i2]) {
            Line::new(p1, p2)
                .into_styled(PrimitiveStyle::with_stroke(BinaryColor::Off, 1))
                .draw(display)?;
        }
    }

    Ok(())
}

#[embassy_executor::task]
pub async fn slab_app(context: AppContext) {
    loop {
        if !context.is_focused().await {
            Timer::after(Duration::from_millis(100)).await;
            continue;
        }

        let q = ORIENTATION_CHANNEL.wait().await;
        let g = get_gravity(&q);
        let ypr = get_yaw_pitch_roll(&q, &g);
        info!("ypr: {}, {}, {}", ypr.0, ypr.1, ypr.2);
        let rotated_direction = q.rotate_vector(Vec3(0.0, 0.0, 1.0));

        context.draw(|canvas| {
            canvas.clear();

            // Define light direction (e.g., coming from top-left-front)
            let light_dir = Vec3(-1.0, 1.0, -1.0);

            if let Err(e) = draw_slab(
                canvas,
                Vec3(0.0, 0.0, 5.0),
                2.0,
                45.0,
                canvas.width(),
                canvas.height(),
                rotated_direction,
                1.0,
                0.5,
                2.0,
                light_dir,
            ) {
                defmt::info!("Draw error: {:?}", e);
            }
        }).await;

        context.request_redraw().await;
        Timer::after(Duration::from_millis(16)).await;
    }
}