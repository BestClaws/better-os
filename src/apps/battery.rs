// Clean bouncing ball implementation
#![allow(unused)]
use core::fmt::Write;
use alloc::vec::Vec;
use defmt::info;
use embassy_time::{Duration, Timer, Instant};
use embedded_graphics::{
    pixelcolor::BinaryColor,
    prelude::*,
    primitives::{Line, PrimitiveStyle, Triangle, Circle},
};
use embedded_graphics::mono_font::ascii::{FONT_6X10, FONT_8X13};
use embedded_graphics::mono_font::MonoTextStyle;
use embedded_graphics::primitives::{Rectangle, Styled, StyledDrawable};
use embedded_graphics::text::Text;
use embedded_graphics_core::pixelcolor::{Gray4, Rgb565};
use micromath::F32Ext;
use crate::system::app::app_context::AppContext;
use crate::system::services::battery_srv::BATTERY_CHANNEL;
use crate::system::services::gyro_accel_srv::ORIENTATION_CHANNEL;
use crate::system::ui::canvas::{Canvas};
use crate::util::math::primitives::Vec3;

const CANVAS_WIDTH: i32 = 320;
const CANVAS_HEIGHT: i32 = 240;
const BALL_RADIUS: i32 = 15;
const GRAVITY: f32 = 300.0;
const BOUNCE_DAMPING: f32 = 0.85;
const AIR_RESISTANCE: f32 = 0.995;

struct Ball {
    position: (f32, f32),
    velocity: (f32, f32),
    radius: i32,
    last_update: Instant,
    color: Rgb565,
}

impl Ball {
    fn new() -> Self {
        Self {
            position: (CANVAS_WIDTH as f32 / 2.0, 50.0),
            velocity: (100.0, -120.0),
            radius: BALL_RADIUS,
            last_update: Instant::now(),
            color: Rgb565::new(31, 15, 15), // Bright red
        }
    }

    fn update(&mut self) {
        let now = Instant::now();
        let dt = (now - self.last_update).as_millis() as f32 / 1000.0;
        self.last_update = now;

        // Cap delta time to prevent large jumps
        let dt = dt.min(0.02);

        // Apply physics
        self.velocity.1 += GRAVITY * dt; // Gravity affects Y velocity
        self.velocity.0 *= AIR_RESISTANCE.powf(dt * 60.0);
        self.velocity.1 *= AIR_RESISTANCE.powf(dt * 60.0);

        // Update position
        self.position.0 += self.velocity.0 * dt;
        self.position.1 += self.velocity.1 * dt;

        // Boundary checking and bouncing
        self.handle_collisions();
        self.update_color();
    }

    fn handle_collisions(&mut self) {
        let r = self.radius as f32;

        // Left/Right walls
        if self.position.0 - r <= 0.0 {
            self.position.0 = r;
            self.velocity.0 = -self.velocity.0 * BOUNCE_DAMPING;
        } else if self.position.0 + r >= CANVAS_WIDTH as f32 {
            self.position.0 = CANVAS_WIDTH as f32 - r;
            self.velocity.0 = -self.velocity.0 * BOUNCE_DAMPING;
        }

        // Top/Bottom walls
        if self.position.1 - r <= 0.0 {
            self.position.1 = r;
            self.velocity.1 = -self.velocity.1 * BOUNCE_DAMPING;
        } else if self.position.1 + r >= CANVAS_HEIGHT as f32 {
            self.position.1 = CANVAS_HEIGHT as f32 - r;
            self.velocity.1 = -self.velocity.1 * BOUNCE_DAMPING;

            // Add energy to prevent settling
            if self.velocity.1.abs() < 30.0 {
                self.velocity.1 = -80.0;
            }
        }
    }

    fn update_color(&mut self) {
        let speed = (self.velocity.0 * self.velocity.0 + self.velocity.1 * self.velocity.1).sqrt();
        let intensity = (speed / 200.0).min(1.0);

        let red = (15 + (intensity * 16.0) as u8).min(31);
        let green = (5 + (intensity * 10.0) as u8).min(63);
        let blue = (5 + (intensity * 10.0) as u8).min(31);

        self.color = Rgb565::new(red, green, blue);
    }

    fn draw(&self, canvas: &mut Canvas<Rgb565>) {
        let center = Point::new(self.position.0 as i32, self.position.1 as i32);

        // Draw shadow/trail
        let trail_offset = Point::new(
            -(self.velocity.0 * 0.008) as i32,
            -(self.velocity.1 * 0.008) as i32
        );
        let trail_center = center + trail_offset;

        if self.is_point_in_bounds(trail_center) {
            let shadow_color = Rgb565::new(5, 2, 2);
            Circle::new(
                trail_center - Point::new(self.radius, self.radius),
                (self.radius * 2) as u32
            )
                .draw_styled(&PrimitiveStyle::with_fill(shadow_color), canvas)
                .ok();
        }

        // Draw main ball
        Circle::new(
            center - Point::new(self.radius, self.radius),
            (self.radius * 2) as u32
        )
            .draw_styled(&PrimitiveStyle::with_fill(self.color), canvas)
            .unwrap();

        // Draw highlight for 3D effect
        let highlight = Circle::new(
            center - Point::new(self.radius, self.radius) + Point::new(self.radius/3, self.radius/3),
            (self.radius / 2) as u32
        );
        highlight
            .draw_styled(&PrimitiveStyle::with_fill(Rgb565::WHITE), canvas)
            .ok();
    }

    fn is_point_in_bounds(&self, point: Point) -> bool {
        point.x >= self.radius &&
            point.x < CANVAS_WIDTH - self.radius &&
            point.y >= self.radius &&
            point.y < CANVAS_HEIGHT - self.radius
    }

    fn get_debug_info(&self) -> heapless::String<64> {
        let mut buf = heapless::String::<64>::new();
        let speed = (self.velocity.0 * self.velocity.0 + self.velocity.1 * self.velocity.1).sqrt();
        write!(buf, "Speed: {:.0} px/s | Pos: ({:.0},{:.0})",
               speed, self.position.0, self.position.1).ok();
        buf
    }
}

#[embassy_executor::task]
pub async fn battery_app(context: AppContext) {
    let mut ball = Ball::new();
    info!("Battery app started with bouncing ball");

    loop {
        if !context.is_focused().await {
            Timer::after(Duration::from_millis(100)).await;
            continue;
        }

        context.draw(|canvas: &mut Canvas<Rgb565>| {
            // Clear screen with dark background
            canvas.clear(Rgb565::BLACK);

            // Update and draw ball
            ball.update();
            ball.draw(canvas);

            // Draw battery indicator
            draw_battery_info(canvas);

            // Draw debug info
            draw_debug_info(canvas, &ball);
        }).await;

        context.request_redraw().await;
        Timer::after(Duration::from_millis(16)).await; // ~60 FPS
    }
}

fn draw_battery_info(canvas: &mut Canvas<Rgb565>) {
    // Battery outline
    let battery_rect = Rectangle::new(Point::new(10, 10), Size::new(60, 20));
    battery_rect
        .draw_styled(&PrimitiveStyle::with_stroke(Rgb565::WHITE, 2), canvas)
        .ok();

    // Battery fill (placeholder - replace with actual battery level)
    let fill_width = 45; // Represents battery percentage
    let battery_fill = Rectangle::new(Point::new(12, 12), Size::new(fill_width, 16));
    battery_fill
        .draw_styled(&PrimitiveStyle::with_fill(Rgb565::GREEN), canvas)
        .ok();

    // Battery terminal
    let terminal = Rectangle::new(Point::new(70, 15), Size::new(4, 10));
    terminal
        .draw_styled(&PrimitiveStyle::with_fill(Rgb565::WHITE), canvas)
        .ok();

    // Battery text
    let text_style = MonoTextStyle::new(&FONT_6X10, Rgb565::WHITE);
    Text::new("Battery", Point::new(10, 45), text_style)
        .draw(canvas)
        .ok();
}

fn draw_debug_info(canvas: &mut Canvas<Rgb565>, ball: &Ball) {
    let debug_info = ball.get_debug_info();
    let debug_style = MonoTextStyle::new(&FONT_6X10, Rgb565::new(20, 20, 20));

    Text::new(&debug_info, Point::new(10, CANVAS_HEIGHT - 20), debug_style)
        .draw(canvas)
        .ok();
}