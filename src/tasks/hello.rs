use core::fmt::Write;
use defmt::info;
use embassy_time::{Timer, Duration};
use embedded_graphics::{
    mono_font::{ascii::FONT_6X10, MonoTextStyle},
    pixelcolor::BinaryColor,
    prelude::*,
    primitives::{Line, PrimitiveStyle, Rectangle},
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

    // Normalized projection to reduce warping
    let x_proj = (v.0 * f) / v.2;
    let y_proj = (v.1 * f) / (v.2 * aspect);

    // Center on screen
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

    // Transform vertex using the new basis
    Vec3(
        v.0 * right.0 + v.1 * new_up.0 + v.2 * dir.0,
        v.0 * right.1 + v.1 * new_up.1 + v.2 * dir.1,
        v.0 * right.2 + v.1 * new_up.2 + v.2 * dir.2,
    )
}

// Define arrow vertices: cuboid body + extruded rectangle, centered at (0,0,0)
pub const ARROW_VERTICES: [Vec3; 12] = [
    // Cuboid body (rectangular prism, centered around z=0)
    Vec3(-0.5, -0.5, -1.5), // 0
    Vec3( 0.5, -0.5, -1.5), // 1
    Vec3( 0.5,  0.5, -1.5), // 2
    Vec3(-0.5,  0.5, -1.5), // 3
    Vec3(-0.5, -0.5,  0.5), // 4
    Vec3( 0.5, -0.5,  0.5), // 5
    Vec3( 0.5,  0.5,  0.5), // 6
    Vec3(-0.5,  0.5,  0.5), // 7
    // Arrowhead (extruded rectangle at z=0.5)
    Vec3(-1.2, -0.7,  0.5), // 8
    Vec3( 1.2, -0.7,  0.5), // 9
    Vec3( 0.7,  0.7,  0.5), // 10
    Vec3(-0.7,  0.7,  0.5), // 11
];

pub const ARROW_EDGES: [(usize, usize); 16] = [
    // Cuboid body edges
    (0, 1), (1, 2), (2, 3), (3, 0),
    (4, 5), (5, 6), (6, 7), (7, 4),
    (0, 4), (1, 5), (2, 6), (3, 7),
    // Arrowhead edges
    (8, 9), (9, 10), (10, 11), (11, 8),
];

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
) {
    let mut projected: [Option<Point>; 12] = [None; 12];

    for (i, &v) in ARROW_VERTICES.iter().enumerate() {
        // Apply rotation first, then orient to direction
        let v = rotate_xyz(v, angle_x, angle_y, angle_z);
        let v = orient_to_direction(v, direction);
        let world = Vec3(
            origin.0 + v.0 * size,
            origin.1 + v.1 * size,
            origin.2 + v.2 * size,
        );

        projected[i] = project(world, fov_deg, width, height)
            .map(|(x, y)| Point::new(x, y));
    }

    for &(i1, i2) in ARROW_EDGES.iter() {
        if let (Some(p1), Some(p2)) = (projected[i1], projected[i2]) {
            let _ = Line::new(p1, p2)
                .into_styled(PrimitiveStyle::with_stroke(BinaryColor::On, 2))
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

        draw_arrow(
            &mut context.canvas,
            Vec3(0.0, 0.0, 5.0), // Z distance for visibility
            2.0,                 // Larger size
            45.0,                // Balanced FOV to reduce warping
            w,
            h,
            direction,
            angle_x,
            angle_y,
            angle_z
        );

        context.request_redraw().await;
        Timer::after(Duration::from_millis(16)).await;
        info!("Hello app tick");
    }
}