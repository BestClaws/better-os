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
}

pub fn project(v: Vec3, fov_deg: f32, width: u32, height: u32) -> Option<(i32, i32)> {
    if v.2 <= 0.1 {
        return None; // behind camera
    }

    let fov_rad = fov_deg.to_radians();
    let aspect = width as f32 / height as f32;

    let x_proj = (v.0 / v.2) * (1.0 / fov_rad.tan());
    let y_proj = (v.1 / v.2) * (1.0 / fov_rad.tan()) / aspect;

    Some((
        ((x_proj + 1.0) * (width as f32 / 2.0)) as i32,
        ((1.0 - y_proj) * (height as f32 / 2.0)) as i32,
    ))
}

pub fn rotate_xyz(v: Vec3, angle_rad: f32) -> Vec3 {
    let (s, c) = angle_rad.sin_cos();

    // Rotate around X
    let y1 = v.1 * c - v.2 * s;
    let z1 = v.1 * s + v.2 * c;
    let v = Vec3(v.0, y1, z1);

    // Rotate around Y
    let x2 = v.0 * c + v.2 * s;
    let z2 = -v.0 * s + v.2 * c;
    let v = Vec3(x2, v.1, z2);

    // Rotate around Z
    let x3 = v.0 * c - v.1 * s;
    let y3 = v.0 * s + v.1 * c;
    Vec3(x3, y3, v.2)
}

pub const CUBE_VERTICES: [Vec3; 8] = [
    Vec3(-1.0, -1.0, -1.0),
    Vec3( 1.0, -1.0, -1.0),
    Vec3( 1.0,  1.0, -1.0),
    Vec3(-1.0,  1.0, -1.0),
    Vec3(-1.0, -1.0,  1.0),
    Vec3( 1.0, -1.0,  1.0),
    Vec3( 1.0,  1.0,  1.0),
    Vec3(-1.0,  1.0,  1.0),
];

pub const CUBE_EDGES: [(usize, usize); 12] = [
    (0, 1), (1, 2), (2, 3), (3, 0),
    (4, 5), (5, 6), (6, 7), (7, 4),
    (0, 4), (1, 5), (2, 6), (3, 7),
];

pub fn draw_cube<D: DrawTarget<Color = BinaryColor>>(
    display: &mut D,
    origin: Vec3,
    size: f32,
    fov_deg: f32,
    width: u32,
    height: u32,
    angle : f32,
) {
    let mut projected: [Option<Point>; 8] = [None; 8];

    for (i, &v) in CUBE_VERTICES.iter().enumerate() {
        let v = rotate_xyz(v, angle);
        let world = Vec3(
            origin.0 + v.0 * size,
            origin.1 + v.1 * size,
            origin.2 + v.2 * size,
        );

        projected[i] = project(world, fov_deg, width, height)
            .map(|(x, y)| Point::new(x, y));
    }

    for &(i1, i2) in CUBE_EDGES.iter() {
        if let (Some(p1), Some(p2)) = (projected[i1], projected[i2]) {
            let _ = Line::new(p1, p2)
                .into_styled(PrimitiveStyle::with_stroke(BinaryColor::On, 1))
                .draw(display);
        }
    }
}

/// Battery app: draws battery percentage and 3D cube onto its canvas.
#[embassy_executor::task]
pub async fn hello_app(mut context: AppContext<'static>) {
    let receiver = BATTERY_CHANNEL.receiver();

    let mut angle = 0.0f32.to_radians();

    loop {
        angle += 0.5;

        if !context.is_focused().await {
            Timer::after(Duration::from_millis(100)).await;
            continue;
        }

        context.canvas.clear();

        let mut text_buf = heapless::String::<32>::new();
        write!(text_buf, "Battery").ok();

        let style = MonoTextStyle::new(&FONT_8X13_BOLD, BinaryColor::On);
        Text::new(&text_buf, Point::new(20, 10), style)
            .draw(&mut context.canvas)
            .unwrap();




        let w = context.width();
        let h = context.height();
        draw_cube(
            &mut context.canvas,
            Vec3(0.0, 0.0, 4.5),
            1.0,
            30.0,
            w,
            h,
            angle
        );



        context.request_redraw().await;
        Timer::after(Duration::from_millis(30)).await;
        info!("Hello app tick");
    }
}
