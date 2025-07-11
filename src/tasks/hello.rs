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

    let x_proj = (v.0 / v.2) * (1.0 / (fov_rad / 2.0).tan());
    let y_proj = (v.1 / v.2) * (1.0 / (fov_rad / 2.0).tan()) / aspect;

    Some((
        ((x_proj + 1.0) * (width as f32 / 2.0)) as i32,
        ((1.0 - y_proj) * (height as f32 / 2.0)) as i32,
    ))
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

// Define arrow vertices: cuboid body + extruded rectangle for arrowhead
pub const ARROW_VERTICES: [Vec3; 12] = [
    // Cuboid body (rectangular prism)
    Vec3(-0.2, -0.2, -1.0), // 0
    Vec3( 0.2, -0.2, -1.0), // 1
    Vec3( 0.2,  0.2, -1.0), // 2
    Vec3(-0.2,  0.2, -1.0), // 3
    Vec3(-0.2, -0.2,  0.0), // 4
    Vec3( 0.2, -0.2,  0.0), // 5
    Vec3( 0.2,  0.2,  0.0), // 6
    Vec3(-0.2,  0.2,  0.0), // 7
    // Arrowhead (extruded rectangle, wider base)
    Vec3(-0.5, -0.3,  0.0), // 8
    Vec3( 0.5, -0.3,  0.0), // 9
    Vec3( 0.3,  0.3,  0.0), // 10
    Vec3(-0.3,  0.3,  0.0), // 11
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
) {
    let mut projected: [Option<Point>; 12] = [None; 12];

    for (i, &v) in ARROW_VERTICES.iter().enumerate() {
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
                .into_styled(PrimitiveStyle::with_stroke(BinaryColor::On, 1))
                .draw(display);
        }
    }
}

#[embassy_executor::task]
pub async fn hello_app(mut context: AppContext<'static>) {
    let receiver = BATTERY_CHANNEL.receiver();

    let mut angle = 0.0f32;

    loop {
        angle += 0.05;

        if !context.is_focused().await {
            Timer::after(Duration::from_millis(100)).await;
            continue;
        }

        context.canvas.clear();

        let w = context.width();
        let h = context.height();

        // Dynamic direction vector (rotating for demonstration)
        let direction = Vec3(
            angle.sin(),
            angle.cos(),
            1.0
        ).normalize();

        draw_arrow(
            &mut context.canvas,
            Vec3(0.0, 0.0, 4.0), // Centered origin
            1.0,                 // Arrow size
            60.0,                // FOV for perspective
            w,
            h,
            direction
        );

        context.request_redraw().await;
        Timer::after(Duration::from_millis(16)).await;
        info!("Hello app tick");
    }
}