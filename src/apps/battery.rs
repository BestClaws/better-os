// Allow unused code for prototyping
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

// Physics-based bouncing circle with proper time delta
struct BouncingCircle {
    x: f32,
    y: f32,
    velocity_x: f32,  // pixels per second
    velocity_y: f32,  // pixels per second
    radius: i32,
    last_update: Instant,
    // Physics constants
    gravity: f32,      // pixels per second²
    bounce_damping: f32, // energy loss on bounce (0.0 = no bounce, 1.0 = perfect bounce)
    air_resistance: f32, // velocity damping factor
}

impl BouncingCircle {
    fn new(x: f32, y: f32, radius: i32) -> Self {
        Self {
            x,
            y,
            velocity_x: 120.0,  // 120 pixels per second
            velocity_y: -150.0, // Start moving upward
            radius,
            last_update: Instant::now(),
            gravity: 400.0,     // Gravity acceleration
            bounce_damping: 0.8, // 80% energy retained on bounce
            air_resistance: 0.99, // Very slight air resistance
        }
    }

    fn update(&mut self, canvas_width: i32, canvas_height: i32) {
        let now = Instant::now();
        let delta_time = (now - self.last_update).as_millis() as f32 / 1000.0; // Convert to seconds
        self.last_update = now;

        // Clamp delta time to prevent large jumps (useful for debugging/pausing)
        let delta_time = delta_time.min(0.016); // Cap at ~60fps equivalent

        // Apply gravity to vertical velocity
        self.velocity_y += self.gravity * delta_time;

        // Apply air resistance
        self.velocity_x *= self.air_resistance.powf(delta_time * 60.0);
        self.velocity_y *= self.air_resistance.powf(delta_time * 60.0);

        // Update position based on velocity and time
        self.x += self.velocity_x * delta_time;
        self.y += self.velocity_y * delta_time;

        // Bounce off walls with damping
        let min_x = self.radius as f32;
        let max_x = (canvas_width - self.radius) as f32;
        let min_y = self.radius as f32;
        let max_y = (canvas_height - self.radius) as f32;

        // Horizontal boundaries
        if self.x <= min_x {
            self.x = min_x;
            self.velocity_x = -self.velocity_x * self.bounce_damping;
        } else if self.x >= max_x {
            self.x = max_x;
            self.velocity_x = -self.velocity_x * self.bounce_damping;
        }

        // Vertical boundaries
        if self.y <= min_y {
            self.y = min_y;
            self.velocity_y = -self.velocity_y * self.bounce_damping;
        } else if self.y >= max_y {
            self.y = max_y;
            self.velocity_y = -self.velocity_y * self.bounce_damping;

            // Add a little randomness to prevent getting stuck in a boring pattern
            if self.velocity_y.abs() < 50.0 {
                self.velocity_y = -100.0; // Give it a little kick upward
                self.velocity_x += (now.as_millis() % 3) as f32 * 20.0 - 30.0; // Add some horizontal randomness
            }
        }
    }

    fn draw(&self, canvas: &mut Canvas<Rgb565>) {
        let center = Point::new(self.x as i32, self.y as i32);

        // Create a subtle trail effect by drawing a slightly transparent circle behind
        let trail_center = Point::new(
            (self.x - self.velocity_x * 0.01) as i32,
            (self.y - self.velocity_y * 0.01) as i32
        );

        // Draw trail (dimmer)
        if trail_center.x >= self.radius && trail_center.x < 320 - self.radius &&
            trail_center.y >= self.radius && trail_center.y < 240 - self.radius {
            Circle::new(
                trail_center - Point::new(self.radius, self.radius),
                self.radius as u32 * 2
            )
                .draw_styled(&PrimitiveStyle::with_fill(Rgb565::new(15, 5, 10)), canvas)
                .ok();
        }

        // Draw main circle with color based on speed
        let speed = (self.velocity_x * self.velocity_x + self.velocity_y * self.velocity_y).sqrt();
        let speed_factor = (speed / 200.0).min(1.0);

        let red = (15.0 + speed_factor * 16.0) as u8;
        let green = (5.0 + speed_factor * 10.0) as u8;
        let blue = (10.0 + speed_factor * 5.0) as u8;

        Circle::new(center - Point::new(self.radius, self.radius), self.radius as u32 * 2)
            .draw_styled(&PrimitiveStyle::with_fill(Rgb565::new(red, green, blue)), canvas)
            .unwrap();
    }

    // Method to reset the circle if it gets stuck or for fun
    fn reset(&mut self, x: f32, y: f32) {
        self.x = x;
        self.y = y;
        self.velocity_x = 120.0;
        self.velocity_y = -150.0;
        self.last_update = Instant::now();
    }
}

#[embassy_executor::task]
pub async fn battery_app(context: AppContext) {
    // Initialize the bouncing circle
    let mut bouncing_circle = BouncingCircle::new(100.0, 50.0, 10);

    loop {
        if !context.is_focused().await {
            Timer::after(Duration::from_millis(100)).await;
            continue;
        }

        context.draw(|mut canvas: &mut Canvas<Rgb565>| {
            canvas.clear(Rgb565::BLACK);

            // Get canvas dimensions (adjust these to match your actual display)
            let canvas_width = 320;
            let canvas_height = 240;

            // Update circle physics (now time-independent!)
            bouncing_circle.update(canvas_width, canvas_height);

            // Draw the bouncing circle with trail effect
            bouncing_circle.draw(canvas);

            // Draw battery info
            let mut text_buf = heapless::String::<32>::new();
            write!(text_buf, "Battery:");

            Rectangle::new(Point::new(50, 200), Size::new(50, 10))
                .draw_styled(&PrimitiveStyle::with_fill(Rgb565::WHITE), canvas)
                .unwrap();

            let style = MonoTextStyle::new(&FONT_8X13, Rgb565::new(5, 20, 50));
            Text::new(&text_buf, Point::new(50, 215), style)
                .draw(canvas)
                .unwrap();

            // Optional: Display physics info for debugging
            let mut debug_buf = heapless::String::<64>::new();
            let speed = (bouncing_circle.velocity_x * bouncing_circle.velocity_x +
                bouncing_circle.velocity_y * bouncing_circle.velocity_y).sqrt();
            write!(debug_buf, "Speed: {:.0} px/s", speed).ok();

            let debug_style = MonoTextStyle::new(&FONT_6X10, Rgb565::new(15, 15, 15));
            Text::new(&debug_buf, Point::new(10, 20), debug_style)
                .draw(canvas)
                .ok();
        }).await;

        context.request_redraw().await;
        Timer::after(Duration::from_millis(1)).await; // Your 1ms loop is now fine!
    }
}