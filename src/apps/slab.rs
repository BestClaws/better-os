#![allow(unused)]


use alloc::vec::Vec;
use defmt::info;
use embassy_time::{Timer, Duration};
use embedded_graphics::{
    pixelcolor::BinaryColor,
    prelude::*,
    primitives::{Triangle, Line, PrimitiveStyle},
};
use micromath::F32Ext;
use crate::system::app::app_context::AppContext;
use crate::system::services::battery_srv::BATTERY_CHANNEL;
use crate::system::services::gyro_accel_srv::ORIENTATION_CHANNEL;
use crate::util::math::primitives::Vec3;

pub fn project(v: Vec3, fov_deg: f32, width: u32, height: u32) -> Option<(i32, i32)> {
    if v.2 <= 0.1 {
        return None; // behind camera
    }

    let fov_rad = fov_deg.to_radians();
    let aspect = width as f32 / height as f32;
    let f = 1.0 / (fov_rad / 2.0).tan(); // Focal length for perspective

    let x_proj = (v.0 * f) / v.2;
    let y_proj = (v.1 * f) / (v.2 * aspect);

    Some((
        ((x_proj + 1.0) * (width as f32 / 2.0)) as i32,
        ((1.0 - y_proj) * (height as f32 / 2.0)) as i32,
    ))
}

pub fn rotate_xyz(v: Vec3, angle_x: f32, angle_y: f32, angle_z: f32) -> Vec3 {
    let (sx, cx) = angle_x.sin_cos();
    let y1 = v.1 * cx - v.2 * sx;
    let z1 = v.1 * sx + v.2 * cx;
    let v = Vec3(v.0, y1, z1);

    let (sy, cy) = angle_y.sin_cos();
    let x2 = v.0 * cy + v.2 * sy;
    let z2 = -v.0 * sy + v.2 * cy;
    let v = Vec3(x2, v.1, z2);

    let (sz, cz) = angle_z.sin_cos();
    let x3 = v.0 * cz - v.1 * sz;
    let y3 = v.0 * sz + v.1 * cz;
    Vec3(x3, y3, v.2)
}

pub fn draw_arrow<D: DrawTarget<Color = BinaryColor>>(
    display: &mut D,
    origin: Vec3,
    size: f32,
    fov_deg: f32,
    width: u32,
    height: u32,
    angles: Vec3,
    slab_width: f32,
    slab_height: f32,
    slab_length: f32,
) {
    let half_width = slab_width / 2.0;
    let half_height = slab_height / 2.0;
    let half_length = slab_length / 2.0;
    let rotation_center_z = 0.0;

    let vertices: [Vec3; 8] = [
        Vec3(-half_width, -half_height, -half_length - rotation_center_z),
        Vec3(half_width, -half_height, -half_length - rotation_center_z),
        Vec3(half_width, half_height, -half_length - rotation_center_z),
        Vec3(-half_width, half_height, -half_length - rotation_center_z),
        Vec3(-half_width, -half_height, half_length - rotation_center_z),
        Vec3(half_width, -half_height, half_length - rotation_center_z),
        Vec3(half_width, half_height, half_length - rotation_center_z),
        Vec3(-half_width, half_height, half_length - rotation_center_z),
    ];

    let faces: [(&[usize], &str); 12] = [
        (&[0, 1, 2], "cuboid_back1"),
        (&[2, 3, 0], "cuboid_back2"),
        (&[4, 5, 6], "cuboid_front1"),
        (&[6, 7, 4], "cuboid_front2"),
        (&[0, 1, 5], "cuboid_bottom1"),
        (&[5, 4, 0], "cuboid_bottom2"),
        (&[2, 3, 7], "cuboid_top1"),
        (&[7, 6, 2], "cuboid_top2"),
        (&[0, 3, 7], "cuboid_left1"),
        (&[7, 4, 0], "cuboid_left2"),
        (&[1, 2, 6], "cuboid_right1"),
        (&[6, 5, 1], "cuboid_right2"),
    ];

    let edges: [(usize, usize); 12] = [
        (0, 1), (1, 2), (2, 3), (3, 0),
        (4, 5), (5, 6), (6, 7), (7, 4),
        (0, 4), (1, 5), (2, 6), (3, 7),
    ];

    let mut projected: [Option<Point>; 8] = [None; 8];
    let mut world_vertices: [Vec3; 8] = [Vec3(0.0, 0.0, 0.0); 8];

    for (i, &v) in vertices.iter().enumerate() {
        let v = rotate_xyz(v, angles.0, angles.1, angles.2);
        let world = Vec3(
            origin.0 + v.0 * size,
            origin.1 + v.1 * size,
            origin.2 + v.2 * size,
        );
        world_vertices[i] = world;
        projected[i] = project(world, fov_deg, width, height).map(Point::from);
    }

    let mut sorted_faces: Vec<(&[usize], &str, f32)> = faces
        .iter()
        .map(|(indices, name)| {
            let avg_z = indices.iter().map(|&i| world_vertices[i].2).sum::<f32>() / indices.len() as f32;
            (*indices, *name, avg_z)
        })
        .collect();

    sorted_faces.sort_by(|a, b| b.2.partial_cmp(&a.2).unwrap_or(core::cmp::Ordering::Equal));

    for (indices, _, _) in sorted_faces {
        if let (Some(p0), Some(p1), Some(p2)) = (
            projected[indices[0]],
            projected[indices[1]],
            projected[indices[2]],
        ) {
            let _ = Triangle::new(p0, p1, p2)
                .into_styled(PrimitiveStyle::with_fill(BinaryColor::On))
                .draw(display);
        }
    }

    for &(i1, i2) in edges.iter() {
        if let (Some(p1), Some(p2)) = (projected[i1], projected[i2]) {
            let _ = Line::new(p1, p2)
                .into_styled(PrimitiveStyle::with_stroke(BinaryColor::Off, 1))
                .draw(display);
        }
    }
}

#[embassy_executor::task]
pub async fn slab_app(mut context: AppContext<'static>) {
    let receiver = ORIENTATION_CHANNEL.receiver();



    loop {


        if !context.is_focused().await {
            Timer::after(Duration::from_millis(100)).await;
            continue;
        }

        context.canvas.clear();

        let w = context.width();
        let h = context.height();

        let slab_width = 1.0;
        let slab_height = 0.5;
        let slab_length = 2.0;


        let v = receiver.receive().await;


        draw_arrow(
            &mut context.canvas,
            Vec3(0.0, 0.0, 8.0),
            2.0,
            45.0,
            w,
            h,
            v,
            slab_width,
            slab_height,
            slab_length,
        );

        context.request_redraw().await;
        Timer::after(Duration::from_millis(1)).await;
    }
}

/// Returns (pitch, yaw, roll) in **radians**
pub fn vector_to_angles(v: Vec3) -> (f32, f32, f32) {
    let v = v.normalize();
    let pitch = (-v.1).asin();
    let yaw = v.0.atan2(v.2);
    let roll = 0.0;
    (pitch, yaw, roll)
}
