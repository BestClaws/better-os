use alloc::vec::Vec;
use embassy_time::{Timer, Duration};
use embedded_graphics::{
    pixelcolor::BinaryColor,
    prelude::*,
    primitives::{Triangle, Line, PrimitiveStyle},
};
use micromath::F32Ext;
use crate::system::app::app_context::AppContext;
use crate::system::services::battery::BATTERY_CHANNEL;

#[derive(Copy, Clone)]
pub struct Vec3(pub f32, pub f32, pub f32);

impl Vec3 {
    pub fn add(self, rhs: Vec3) -> Vec3 {
        Vec3(self.0 + rhs.0, self.1 + rhs.1, self.2 + rhs.2)
    }

    pub fn sub(self, rhs: Vec3) -> Vec3 {
        Vec3(self.0 - rhs.0, self.1 - rhs.1, self.2 - rhs.2)
    }

    pub fn dot(self, rhs: Vec3) -> f32 {
        self.0 * rhs.0 + self.1 * rhs.1 + self.2 * rhs.2
    }

    pub fn cross(self, rhs: Vec3) -> Vec3 {
        Vec3(
            self.1 * rhs.2 - self.2 * rhs.1,
            self.2 * rhs.0 - self.0 * rhs.2,
            self.0 * rhs.1 - self.1 * rhs.0,
        )
    }

    pub fn scale(self, s: f32) -> Vec3 {
        Vec3(self.0 * s, self.1 * s, self.2 * s)
    }

    pub fn normalize(self) -> Vec3 {
        let mag = (self.0 * self.0 + self.1 * self.1 + self.2 * self.2).sqrt();
        if mag > 0.0 {
            self.scale(1.0 / mag)
        } else {
            self
        }
    }
}

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
    // Rotate around X axis
    let (sx, cx) = angle_x.sin_cos();
    let y1 = v.1 * cx - v.2 * sx;
    let z1 = v.1 * sx + v.2 * cx;
    let v = Vec3(v.0, y1, z1);

    // Rotate around Y axis
    let (sy, cy) = angle_y.sin_cos();
    let x2 = v.0 * cy + v.2 * sy;
    let z2 = -v.0 * sy + v.2 * cy;
    let v = Vec3(x2, v.1, z2);

    // Rotate around Z axis
    let (sz, cz) = angle_z.sin_cos();
    let x3 = v.0 * cz - v.1 * sz;
    let y3 = v.0 * sz + v.1 * cz;
    Vec3(x3, y3, v.2)
}

pub fn orient_to_direction(v: Vec3, dir: Vec3) -> Vec3 {
    let dir = dir.normalize();
    let up = Vec3(0.0, 1.0, 0.0);
    let right = dir.cross(up).normalize();
    let new_up = right.cross(dir).normalize();

    Vec3(
        v.0 * right.0 + v.1 * new_up.0 + v.2 * dir.0,
        v.0 * right.1 + v.1 * new_up.1 + v.2 * dir.1,
        v.0 * right.2 + v.1 * new_up.2 + v.2 * dir.2,
    )
}

pub fn draw_arrow<D: DrawTarget<Color = BinaryColor>>(
    display: &mut D,
    origin: Vec3,
    size: f32,
    fov_deg: f32,
    width: u32,
    height: u32,
    direction: Vec3,
    angle_x: f32,
    angle_y: f32,
    angle_z: f32,
    slab_width: f32,
    slab_height: f32,
    slab_length: f32,
) {
    // Define vertices: cuboid (slab)
    let half_width = slab_width / 2.0;
    let half_height = slab_height / 2.0;
    let half_length = slab_length / 2.0;
    let rotation_center_z = 0.0; // Center of cuboid

    let vertices: [Vec3; 8] = [
        Vec3(-half_width, -half_height, -half_length - rotation_center_z), // 0 (back bottom left)
        Vec3(half_width, -half_height, -half_length - rotation_center_z),  // 1 (back bottom right)
        Vec3(half_width, half_height, -half_length - rotation_center_z),   // 2 (back top right)
        Vec3(-half_width, half_height, -half_length - rotation_center_z),  // 3 (back top left)
        Vec3(-half_width, -half_height, half_length - rotation_center_z),  // 4 (front bottom left)
        Vec3(half_width, -half_height, half_length - rotation_center_z),   // 5 (front bottom right)
        Vec3(half_width, half_height, half_length - rotation_center_z),    // 6 (front top right)
        Vec3(-half_width, half_height, half_length - rotation_center_z),   // 7 (front top left)
    ];

    // Define faces for cuboid (6 rectangles as 12 triangles)
    let faces: [(&[usize], &str); 12] = [
        (&[0, 1, 2], "cuboid_back1"),    // Back face (triangle 1)
        (&[2, 3, 0], "cuboid_back2"),    // Back face (triangle 2)
        (&[4, 5, 6], "cuboid_front1"),   // Front face (triangle 1)
        (&[6, 7, 4], "cuboid_front2"),   // Front face (triangle 2)
        (&[0, 1, 5], "cuboid_bottom1"),  // Bottom face (triangle 1)
        (&[5, 4, 0], "cuboid_bottom2"),  // Bottom face (triangle 2)
        (&[2, 3, 7], "cuboid_top1"),     // Top face (triangle 1)
        (&[7, 6, 2], "cuboid_top2"),     // Top face (triangle 2)
        (&[0, 3, 7], "cuboid_left1"),    // Left face (triangle 1)
        (&[7, 4, 0], "cuboid_left2"),    // Left face (triangle 2)
        (&[1, 2, 6], "cuboid_right1"),   // Right face (triangle 1)
        (&[6, 5, 1], "cuboid_right2"),   // Right face (triangle 2)
    ];

    // Define edges for cuboid (12 edges)
    let edges: [(usize, usize); 12] = [
        (0, 1), (1, 2), (2, 3), (3, 0), // Back face
        (4, 5), (5, 6), (6, 7), (7, 4), // Front face
        (0, 4), (1, 5), (2, 6), (3, 7), // Connecting edges
    ];

    let mut projected: [Option<Point>; 8] = [None; 8];
    let mut world_vertices: [Vec3; 8] = [Vec3(0.0, 0.0, 0.0); 8];

    // Transform and project vertices
    for (i, &v) in vertices.iter().enumerate() {
        let v = rotate_xyz(v, angle_x, angle_y, angle_z);
        let v = orient_to_direction(v, direction);
        let world = Vec3(
            origin.0 + v.0 * size,
            origin.1 + v.1 * size,
            origin.2 + v.2 * size,
        );
        world_vertices[i] = world;
        projected[i] = project(world, fov_deg, width, height)
            .map(|(x, y)| Point::new(x, y));
    }

    // Sort faces by average Z-depth (back to front)
    let mut sorted_faces: Vec<(&[usize], &str, f32)> = faces
        .iter()
        .map(|(indices, name)| {
            let avg_z = indices
                .iter()
                .map(|&i| world_vertices[i].2)
                .sum::<f32>()
                / indices.len() as f32;
            (*indices, *name, avg_z)
        })
        .collect();

    sorted_faces.sort_by(|a, b| b.2.partial_cmp(&a.2).unwrap_or(core::cmp::Ordering::Equal));

    // Draw filled faces
    for (indices, _, _) in sorted_faces {
        if let (Some(p0), Some(p1), Some(p2)) =
            (projected[indices[0]], projected[indices[1]], projected[indices[2]])
        {
            let _ = Triangle::new(p0, p1, p2)
                .into_styled(PrimitiveStyle::with_fill(BinaryColor::On))
                .draw(display);
        }
    }

    // Draw edges with BinaryColor::Off
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
    let receiver = BATTERY_CHANNEL.receiver();

    let mut angle_x = 0.0f32;
    let mut angle_y = 0.0f32;
    let mut angle_z = 0.0f32;

    loop {
        // Increment angles for smooth rotation
        angle_x += 0.03;
        angle_y += 0.03;
        angle_z += 0.03;

        if !context.is_focused().await {
            Timer::after(Duration::from_millis(100)).await;
            continue;
        }

        context.canvas.clear();

        let w = context.width();
        let h = context.height();

        // Base direction (can be modified for navigation)
        let direction = Vec3(0.0, 0.0, 1.0).normalize();

        // Configurable parameters
        let slab_width = 1.0;
        let slab_height = 0.5;
        let slab_length = 2.0;

        draw_arrow(
            &mut context.canvas,
            Vec3(0.0, 0.0, 8.0), // Z distance for visibility
            2.0,                 // Size for large arrow
            45.0,                // Balanced FOV to reduce warping
            w,
            h,
            direction,
            angle_x,
            angle_y,
            angle_z,
            slab_width,
            slab_height,
            slab_length,
        );

        context.request_redraw().await;
        Timer::after(Duration::from_millis(1)).await;
    }
}