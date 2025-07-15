// Allow unused code for prototyping
#![allow(unused)]

use alloc::vec::Vec;
use defmt::info;
use embassy_time::{Duration, Timer};
use embedded_graphics::{
    pixelcolor::BinaryColor,
    prelude::*,
    primitives::{Line, PrimitiveStyle, Triangle},
};
use micromath::F32Ext;
use crate::system::app::app_context::AppContext;
use crate::system::services::battery_srv::BATTERY_CHANNEL;
use crate::system::services::gyro_accel_srv::ORIENTATION_CHANNEL;
use crate::util::math::primitives::Vec3;

// Projects a 3D point to 2D screen coordinates
fn project(v: Vec3, fov_deg: f32, width: u32, height: u32) -> Option<(i32, i32)> {
    // Ensure point is in front of camera (z > 0.1)
    if v.2 <= 0.1 {
        return None;
    }

    let fov_rad = fov_deg.to_radians();
    let aspect = width as f32 / height as f32;
    let f = 1.0 / (fov_rad / 2.0).tan(); // Focal length for perspective projection

    // Project to screen coordinates
    // X-axis: positive to right, Y-axis: positive up, Z-axis: positive into screen
    let x_proj = (v.0 * f) / v.2;
    let y_proj = (v.1 * f) / (v.2 * aspect);

    // Map to screen pixels (origin at center)
    Some((
        ((x_proj + 1.0) * (width as f32 / 2.0)) as i32,
        ((1.0 - y_proj) * (height as f32 / 2.0)) as i32,
    ))
}

// Computes a rotation matrix to align the slab's Z-axis with a target direction
fn align_to_direction(v: Vec3, target: Vec3) -> [[f32; 3]; 3] {
    // Normalize input vectors
    let v = {
        let mag = (v.0 * v.0 + v.1 * v.1 + v.2 * v.2).sqrt();
        if mag < 0.0001 {
            Vec3(0.0, 0.0, 1.0) // Default to Z-axis if zero vector
        } else {
            Vec3(v.0 / mag, v.1 / mag, v.2 / mag)
        }
    };
    let target = {
        let mag = (target.0 * target.0 + target.1 * target.1 + target.2 * target.2).sqrt();
        if mag < 0.0001 {
            Vec3(0.0, 0.0, 1.0) // Default to Z-axis if zero vector
        } else {
            Vec3(target.0 / mag, target.1 / mag, target.2 / mag)
        }
    };

    // Compute rotation axis (cross product) and angle (dot product)
    let axis = Vec3(
        v.1 * target.2 - v.2 * target.1,
        v.2 * target.0 - v.0 * target.2,
        v.0 * target.1 - v.1 * target.0,
    );
    let axis_mag = (axis.0 * axis.0 + axis.1 * axis.1 + axis.2 * axis.2).sqrt();
    let dot = v.0 * target.0 + v.1 * target.1 + v.2 * target.2;
    let angle = dot.clamp(-1.0, 1.0).acos();

    // If vectors are aligned, return identity matrix
    if axis_mag < 0.0001 || angle < 0.0001 {
        return [
            [1.0, 0.0, 0.0],
            [0.0, 1.0, 0.0],
            [0.0, 0.0, 1.0],
        ];
    }

    // Normalize rotation axis
    let axis = Vec3(axis.0 / axis_mag, axis.1 / axis_mag, axis.2 / axis_mag);
    let (s, c) = angle.sin_cos();
    let one_minus_c = 1.0 - c;

    // Rotation matrix from axis-angle formula
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

// Draws a 3D slab (cuboid) on the display
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

    // Define triangular faces for rendering (two triangles per face)
    let faces = [
        [0, 1, 2], // Back face 1
        [2, 3, 0], // Back face 2
        [4, 5, 6], // Front face 1
        [6, 7, 4], // Front face 2
        [0, 1, 5], // Bottom face 1
        [5, 4, 0], // Bottom face 2
        [2, 3, 7], // Top face 1
        [7, 6, 2], // Top face 2
        [0, 3, 7], // Left face 1
        [7, 4, 0], // Left face 2
        [1, 2, 6], // Right face 1
        [6, 5, 1], // Right face 2
    ];

    // Compute rotation matrix to align slab's Z-axis with direction
    let rotation = align_to_direction(Vec3(0.0, 0.0, 1.0), direction);

    // Transform and project vertices
    let mut projected = [None; 8];
    for (i, &v) in vertices.iter().enumerate() {
        // Rotate vertex
        let rotated = apply_rotation(v, &rotation);
        // Translate to world position and scale
        let world = Vec3(
            origin.0 + rotated.0 * size,
            origin.1 + rotated.1 * size,
            origin.2 + rotated.2 * size,
        );
        projected[i] = project(world, fov_deg, width, height).map(Point::from);
    }

    // Draw filled triangles for all faces (no sorting)
    for indices in faces.iter() {
        if let (Some(p0), Some(p1), Some(p2)) = (
            projected[indices[0]],
            projected[indices[1]],
            projected[indices[2]],
        ) {
            Triangle::new(p0, p1, p2)
                .into_styled(PrimitiveStyle::with_fill(BinaryColor::On))
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

// Main application task to render the slab based on orientation input
#[embassy_executor::task]
pub async fn slab_app(mut context: AppContext<'static>) {
    let receiver = ORIENTATION_CHANNEL.receiver();

    loop {
        // Wait for orientation update
        let direction = receiver.receive().await;

        // Skip rendering if app is not focused
        if !context.is_focused().await {
            Timer::after(Duration::from_millis(100)).await;
            continue;
        }

        // Get display dimensions
        let width = context.width();
        let height = context.height();

        // Define slab dimensions
        let slab_width = 1.0;
        let slab_height = 0.5;
        let slab_length = 2.0;

        // Clear canvas and draw slab
        context.canvas.clear();
        if let Err(e) = draw_slab(
            &mut context.canvas,
            Vec3(0.0, 0.0, 5.0), // Position slab in front of camera
            2.0,                 // Scale
            45.0,                // Field of view
            width,
            height,
            direction,           // Use orientation as direction vector
            slab_width,
            slab_height,
            slab_length,
        ) {
            info!("Draw error: {:?}", e);
        }

        // Request redraw and wait briefly
        context.request_redraw().await;
        Timer::after(Duration::from_millis(16)).await; // ~60 FPS
    }
}