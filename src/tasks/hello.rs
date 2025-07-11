use alloc::vec::Vec;
use core::fmt::Write;
use defmt::info;
use embassy_time::{Timer, Duration};
use embedded_graphics::{
    mono_font::{ascii::FONT_6X10, MonoTextStyle},
    pixelcolor::BinaryColor,
    prelude::*,
    primitives::{Line, PrimitiveStyle, Triangle},
    text::Text,
};
use embedded_graphics::mono_font::iso_8859_16::FONT_8X13_BOLD;
use micromath::F32Ext;
use crate::system::apps::app_context::AppContext;
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

// Define vertices dynamically in draw_arrow
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
    body_width: f32,
    body_height: f32,
    body_length: f32,
) {
    // Define vertices: body cuboid + head cuboid
    let half_width = body_width / 2.0;
    let half_height = body_height / 2.0;
    let half_length = body_length / 2.0;
    let rotation_center_z = -body_length / 4.0; // Three-fourths from back to front

    // Body cuboid: z from -body_length/2 to body_length/2, centered at z=-body_length/4
    // Head cuboid: attached at z=body_length/2, extends in Y (perpendicular), smaller size
    let head_width = body_width * 0.8; // Slightly smaller than body
    let head_height = body_height * 1.5; // Taller in Y for arrowhead
    let head_length = body_width * 0.5; // Short in Z
    let half_head_width = head_width / 2.0;
    let half_head_height = head_height / 2.0;
    let half_head_length = head_length / 2.0;

    let vertices: [Vec3; 16] = [
        // Body cuboid (vertices 0-7)
        Vec3(-half_width, -half_height, -half_length - rotation_center_z), // 0 (back bottom left)
        Vec3( half_width, -half_height, -half_length - rotation_center_z), // 1 (back bottom right)
        Vec3( half_width,  half_height, -half_length - rotation_center_z), // 2 (back top right)
        Vec3(-half_width,  half_height, -half_length - rotation_center_z), // 3 (back top left)
        Vec3(-half_width, -half_height,  half_length - rotation_center_z), // 4 (front bottom left)
        Vec3( half_width, -half_height,  half_length - rotation_center_z), // 5 (front bottom right)
        Vec3( half_width,  half_height,  half_length - rotation_center_z), // 6 (front top right)
        Vec3(-half_width,  half_height,  half_length - rotation_center_z), // 7 (front top left)
        // Head cuboid (vertices 8-15, centered at z=body_length/2, extends in Y)
        Vec3(-half_head_width, -half_head_height, body_length/2.0 - rotation_center_z - half_head_length), // 8
        Vec3( half_head_width, -half_head_height, body_length/2.0 - rotation_center_z - half_head_length), // 9
        Vec3( half_head_width,  half_head_height, body_length/2.0 - rotation_center_z - half_head_length), // 10
        Vec3(-half_head_width,  half_head_height, body_length/2.0 - rotation_center_z - half_head_length), // 11
        Vec3(-half_head_width, -half_head_height, body_length/2.0 - rotation_center_z + half_head_length), // 12
        Vec3( half_head_width, -half_head_height, body_length/2.0 - rotation_center_z + half_head_length), // 13
        Vec3( half_head_width,  half_head_height, body_length/2.0 - rotation_center_z + half_head_length), // 14
        Vec3(-half_head_width,  half_head_height, body_length/2.0 - rotation_center_z + half_head_length), // 15
    ];

    // Define faces with patterns
    let faces: [(&[usize], &str); 12] = [
        // Body cuboid faces
        (&[0, 1, 2, 3], "body_back"),    // Vertical lines
        (&[4, 5, 6, 7], "body_front"),   // Horizontal lines
        (&[0, 1, 5, 4], "body_bottom"),  // Crosshatch
        (&[2, 3, 7, 6], "body_top"),     // Diagonal lines (top-left to bottom-right)
        (&[0, 3, 7, 4], "body_left"),    // Diagonal lines (bottom-left to top-right)
        (&[1, 2, 6, 5], "body_right"),   // Vertical lines
        // Head cuboid faces
        (&[8, 9, 10, 11], "head_back"),  // Vertical lines
        (&[12, 13, 14, 15], "head_front"), // Horizontal lines
        (&[8, 9, 13, 12], "head_bottom"), // Crosshatch
        (&[10, 11, 15, 14], "head_top"),  // Diagonal lines (top-left to bottom-right)
        (&[8, 11, 15, 12], "head_left"),  // Diagonal lines (bottom-left to top-right)
        (&[9, 10, 14, 13], "head_right"), // Vertical lines
    ];

    let mut projected: [Option<Point>; 16] = [None; 16];
    let mut world_vertices: [Vec3; 16] = [Vec3(0.0, 0.0, 0.0); 16];

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

    // Draw patterned faces
    for (indices, name, _) in sorted_faces {
        if let (Some(p0), Some(p1), Some(p2), Some(p3)) =
            (projected[indices[0]], projected[indices[1]], projected[indices[2]], projected[indices[3]])
        {
            // Compute bounding box for the face
            let min_x = p0.x.min(p1.x).min(p2.x).min(p3.x);
            let max_x = p0.x.max(p1.x).max(p2.x).max(p3.x);
            let min_y = p0.y.min(p1.y).min(p2.y).min(p3.y);
            let max_y = p0.y.max(p1.y).max(p2.y).max(p3.y);

            // Draw pattern based on face name
            match name {
                name if name.contains("vertical") => {
                    // Vertical lines
                    let spacing = 5;
                    for x in (min_x..=max_x).step_by(spacing) {
                        let _ = Line::new(Point::new(x, min_y), Point::new(x, max_y))
                            .into_styled(PrimitiveStyle::with_stroke(BinaryColor::On, 1))
                            .draw(display);
                    }
                }
                name if name.contains("horizontal") => {
                    // Horizontal lines
                    let spacing = 5;
                    for y in (min_y..=max_y).step_by(spacing) {
                        let _ = Line::new(Point::new(min_x, y), Point::new(max_x, y))
                            .into_styled(PrimitiveStyle::with_stroke(BinaryColor::On, 1))
                            .draw(display);
                    }
                }
                name if name.contains("crosshatch") => {
                    // Crosshatch (vertical + horizontal)
                    let spacing = 5;
                    for x in (min_x..=max_x).step_by(spacing) {
                        let _ = Line::new(Point::new(x, min_y), Point::new(x, max_y))
                            .into_styled(PrimitiveStyle::with_stroke(BinaryColor::On, 1))
                            .draw(display);
                    }
                    for y in (min_y..=max_y).step_by(spacing) {
                        let _ = Line::new(Point::new(min_x, y), Point::new(max_x, y))
                            .into_styled(PrimitiveStyle::with_stroke(BinaryColor::On, 1))
                            .draw(display);
                    }
                }
                name if name.contains("diagonal") => {
                    // Diagonal lines (top-left to bottom-right)
                    let spacing = 5;
                    let diag_length = ((max_x - min_x).pow(2) as f32 + ((max_y - min_y).pow(2)) as f32 ).sqrt() as i32;
                    for d in (-diag_length..=diag_length).step_by(spacing) {
                        let x_start = min_x + d;
                        let y_start = min_y;
                        let x_end = min_x + d + (max_y - min_y);
                        let y_end = max_y;
                        if x_start <= max_x && x_end >= min_x {
                            let _ = Line::new(
                                Point::new(x_start.max(min_x), y_start.max(min_y)),
                                Point::new(x_end.min(max_x), y_end.min(max_y)),
                            )
                                .into_styled(PrimitiveStyle::with_stroke(BinaryColor::On, 1))
                                .draw(display);
                        }
                    }
                }
                _ => {} // Fallback (no pattern)
            }

            // Draw outline as two triangles for clarity
            let _ = Triangle::new(p0, p1, p2)
                .into_styled(PrimitiveStyle::with_stroke(BinaryColor::On, 1))
                .draw(display);
            let _ = Triangle::new(p2, p3, p0)
                .into_styled(PrimitiveStyle::with_stroke(BinaryColor::On, 1))
                .draw(display);
        }
    }
}

#[embassy_executor::task]
pub async fn hello_app(mut context: AppContext<'static>) {
    let receiver = BATTERY_CHANNEL.receiver();

    let mut angle_x = 0.0f32;
    let mut angle_y = 0.0f32;
    let mut angle_z = 0.0f32;

    loop {
        // Increment angles for smooth rotation
        angle_x += 0.03;
        angle_y += 0.04;
        angle_z += 0.02;

        if !context.is_focused().await {
            Timer::after(Duration::from_millis(100)).await;
            continue;
        }

        context.canvas.clear();

        let w = context.width();
        let h = context.height();

        // Base direction (can be modified for navigation)
        let direction = Vec3(0.0, 0.0, 1.0).normalize();

        // Configurable body dimensions
        let body_width = 1.0;
        let body_height = 1.0;
        let body_length = 2.0;

        draw_arrow(
            &mut context.canvas,
            Vec3(0.0, 0.0, 8.0), // Z distance for visibility
            1.0,                 // Size for large arrow
            45.0,                // Balanced FOV to reduce warping
            w,
            h,
            direction,
            angle_x,
            angle_y,
            angle_z,
            body_width,
            body_height,
            body_length,
        );

        context.request_redraw().await;
        Timer::after(Duration::from_millis(16)).await;
        info!("Hello app tick");
    }
}