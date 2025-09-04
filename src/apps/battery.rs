// Fixed bouncing ball with animated battery
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
use embedded_graphics::mono_font::ascii::FONT_4X6;
use embedded_graphics::mono_font::iso_8859_1::FONT_6X10;
use embedded_graphics::mono_font::MonoTextStyle;
use embedded_graphics::primitives::{Rectangle, Styled, StyledDrawable};
use embedded_graphics::text::Text;
use embedded_graphics_core::pixelcolor::{Gray4, Rgb565};
use micromath::F32Ext;
use crate::system::app::app_context::AppContext;
use crate::system::services::battery_srv::BATTERY_CHANNEL;
use crate::system::ui::canvas::{Canvas};
use crate::util::math::primitives::Vec3;

const CANVAS_WIDTH: i32 = 320;
const CANVAS_HEIGHT: i32 = 240;

struct Ball {
    x: f32,
    y: f32,
    vx: f32,
    vy: f32,
    radius: f32,
}

impl Ball {
    fn new() -> Self {
        Self {
            x: 160.0,  // Center of screen
            y: 60.0,   // Near top
            vx: 80.0,  // Moving right
            vy: 0.0,   // No initial vertical velocity
            radius: 12.0,
        }
    }

    fn update(&mut self, dt: f32) {
        // Apply gravity
        self.vy += 200.0 * dt;

        // Update position
        self.x += self.vx * dt;
        self.y += self.vy * dt;

        // Bounce off walls
        if self.x - self.radius <= 0.0 {
            self.x = self.radius;
            self.vx = -self.vx * 0.8;
        }
        if self.x + self.radius >= CANVAS_WIDTH as f32 {
            self.x = CANVAS_WIDTH as f32 - self.radius;
            self.vx = -self.vx * 0.8;
        }
        if self.y - self.radius <= 0.0 {
            self.y = self.radius;
            self.vy = -self.vy * 0.8;
        }
        if self.y + self.radius >= CANVAS_HEIGHT as f32 {
            self.y = CANVAS_HEIGHT as f32 - self.radius;
            self.vy = -self.vy * 0.8;
        }

        // Add some energy if too slow
        if self.vy.abs() < 10.0 && self.y + self.radius >= CANVAS_HEIGHT as f32 - 1.0 {
            self.vy = -100.0;
        }
    }

    fn draw(&self, canvas: &mut Canvas<Rgb565>) {
        let center = Point::new(self.x as i32, self.y as i32);
        let r = self.radius as i32;

        // Draw the ball with a bright color
        Circle::new(center - Point::new(r, r), (r * 2) as u32)
            .draw_styled(&PrimitiveStyle::with_fill(Rgb565::RED), canvas)
            .ok();

        // Draw a white highlight
        let highlight_center = center - Point::new(r/3, r/3);
        Circle::new(highlight_center - Point::new(r/4, r/4), (r/2) as u32)
            .draw_styled(&PrimitiveStyle::with_fill(Rgb565::WHITE), canvas)
            .ok();
    }
}

struct BatteryAnimation {
    level: f32,          // Current battery level (0.0 to 1.0)
    target_level: f32,   // Target battery level
    last_update: Instant,
    pulse_time: f32,     // For pulsing effect
}

impl BatteryAnimation {
    fn new() -> Self {
        Self {
            level: 0.7,
            target_level: 0.7,
            last_update: Instant::now(),
            pulse_time: 0.0,
        }
    }

    fn update(&mut self) {
        let now = Instant::now();
        let dt = (now - self.last_update).as_millis() as f32 / 1000.0;
        self.last_update = now;

        // Smooth interpolation to target
        let diff = self.target_level - self.level;
        self.level += diff * dt * 3.0; // Adjust speed here

        // Update pulse animation
        self.pulse_time += dt * 2.0;

        // Simulate battery drain
        self.target_level -= dt * 0.02; // Slowly drain
        if self.target_level < 0.1 {
            self.target_level = 0.9; // Reset for demo
        }
    }

    fn draw(&self, canvas: &mut Canvas<Rgb565>) {
        let x = 20;
        let y = 20;
        let width = 80;
        let height = 20;

        // Battery outline
        Rectangle::new(Point::new(x, y), Size::new(width, height))
            .draw_styled(&PrimitiveStyle::with_stroke(Rgb565::WHITE, 1), canvas)
            .ok();

        // Battery terminal
        Rectangle::new(Point::new(x + width as i32, y + 5), Size::new(4, height - 10))
            .draw_styled(&PrimitiveStyle::with_fill(Rgb565::WHITE), canvas)
            .ok();

        // Calculate fill width and color
        let fill_width = (self.level * (width - 4) as f32) as u32;
        let pulse_intensity = (self.pulse_time.sin() * 0.3 + 0.7).max(0.4).min(1.0);

        let color = if self.level > 0.5 {
            // Green when high
            Rgb565::new(0, (31.0 * pulse_intensity) as u8, 0)
        } else if self.level > 0.2 {
            // Yellow when medium
            Rgb565::new((31.0 * pulse_intensity) as u8, (31.0 * pulse_intensity) as u8, 0)
        } else {
            // Red when low (with more pulsing)
            let low_pulse = (self.pulse_time * 3.0).sin() * 0.5 + 0.5;
            Rgb565::new((31.0 * low_pulse) as u8, 0, 0)
        };

        // Battery fill
        if fill_width > 0 {
            Rectangle::new(Point::new(x + 2, y + 2), Size::new(fill_width, height - 4))
                .draw_styled(&PrimitiveStyle::with_fill(color), canvas)
                .ok();
        }

        // Battery percentage text
        let mut text_buf = heapless::String::<16>::new();
        write!(text_buf, "{}%", (self.level * 100.0) as u8).ok();

        let text_style = MonoTextStyle::new(&FONT_4X6, Rgb565::WHITE);
        Text::new(&text_buf, Point::new(x, y + height as i32 + 15), text_style)
            .draw(canvas)
            .ok();
    }
}

#[embassy_executor::task]
pub async fn battery_app(context: AppContext) {
    let mut ball = Ball::new();
    let mut battery = BatteryAnimation::new();
    let mut last_time = Instant::now();

    info!("Battery app started - Ball should be visible!");

    loop {
        if !context.is_focused().await {
            Timer::after(Duration::from_millis(100)).await;
            continue;
        }

        // Calculate delta time
        let now = Instant::now();
        let dt = (now - last_time).as_millis() as f32 / 1000.0;
        let dt = dt.min(0.02); // Cap at 50fps equivalent
        last_time = now;

        context.draw(|canvas: &mut Canvas<Rgb565>| {
            // Clear with black background
            canvas.clear(Rgb565::BLACK);

            // Update and draw ball
            ball.update(dt);
            ball.draw(canvas);

            // Update and draw battery
            battery.update();
            battery.draw(canvas);

            // Draw some debug info
            let mut debug_buf = heapless::String::<64>::new();
            write!(debug_buf, "Ball: ({:.0},{:.0}) Speed: ({:.0},{:.0})",
                   ball.x, ball.y, ball.vx, ball.vy).ok();

            let debug_style = MonoTextStyle::new(&FONT_6X10, Rgb565::new(15, 15, 15));
            Text::new(&debug_buf, Point::new(10, CANVAS_HEIGHT - 15), debug_style)
                .draw(canvas)
                .ok();

            // Draw title
            let title_style = MonoTextStyle::new(&FONT_6X10, Rgb565::CYAN);
            Text::new("Battery Demo", Point::new(200, 30), title_style)
                .draw(canvas)
                .ok();
        }).await;

        context.request_redraw().await;
        Timer::after(Duration::from_millis(16)).await; // 60 FPS
    }
}