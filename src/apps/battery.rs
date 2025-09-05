// Fixed bouncing ball with animated battery
#![allow(unused)]
use core::fmt::Write;
use alloc::vec::Vec;
use defmt::info;
use embassy_time::{Duration, Timer, Instant};
use crate::system::ui::gfx::{Point as GPoint, Size as GSize, Rect as GRect, Rgb565, Rgba8888, draw_line_aa, draw_line_rgba_aa, draw_circle_aa, fill_rect, draw_rect_outline_aa, fill_circle, fill_rect_linear_gradient, LinearGradient, draw_arc_aa, fill_rounded_rect, fill_rect_rgba};
use micromath::F32Ext;
use crate::system::app::app_context::AppContext;
use crate::system::services::battery_srv::BATTERY_CHANNEL;
use crate::system::ui::canvas::Canvas;
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

    fn draw(&self, canvas: &mut Canvas) {
        let center = GPoint::new(self.x as i32, self.y as i32);
        let r = self.radius as i32;
        fill_circle(canvas, center, r, Rgb565::from_rgb(220, 40, 40));
        let highlight_center = GPoint::new(center.x - r/3, center.y - r/3);
        fill_circle(canvas, highlight_center, r/4, Rgb565::WHITE);
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

    fn draw(&self, canvas: &mut Canvas) {
        let x = 20;
        let y = 20;
        let width = 80;
        let height = 20;
        // Battery outline and terminal
        draw_rect_outline_aa(canvas, GRect::new(GPoint::new(x, y), GSize::new(width as u32, height as u32)), 1, Rgb565::WHITE);
        fill_rect(canvas, GRect::new(GPoint::new(x + width as i32, y + 5), GSize::new(4, (height - 10) as u32)), Rgb565::WHITE);

        // Calculate fill width and color
        let fill_width = (self.level * (width - 4) as f32) as u32;
        let pulse_intensity = (self.pulse_time.sin() * 0.3 + 0.7).max(0.4).min(1.0);

        let color = if self.level > 0.5 {
            // Green when high
            Rgb565::from_rgb(0, (255.0 * pulse_intensity) as u8, 0)
        } else if self.level > 0.2 {
            Rgb565::from_rgb((255.0 * pulse_intensity) as u8, (255.0 * pulse_intensity) as u8, 0)
        } else {
            // Red when low (with more pulsing)
            let low_pulse = (self.pulse_time * 3.0).sin() * 0.5 + 0.5;
            Rgb565::from_rgb((255.0 * low_pulse) as u8, 0, 0)
        };

        // Battery fill with gradient and gloss line
        if fill_width > 0 {
            fill_rect(canvas, GRect::new(GPoint::new(x + 2, y + 2), GSize::new(fill_width, (height - 4) as u32)), color);
            let grad = LinearGradient { start: GPoint::new(x + 2, y + 2), end: GPoint::new(x + 2, y + height as i32 - 2), start_color: Rgb565::from_rgb(255, 255, 255), end_color: color };
            fill_rect_linear_gradient(canvas, GRect::new(GPoint::new(x + 2, y + 2), GSize::new(fill_width, (height - 4) as u32)), &grad);
            draw_line_rgba_aa(canvas, GPoint::new(x + 2, y + 2), GPoint::new(x + 2 + fill_width as i32 - 1, y + 2), Rgba8888::new(255, 255, 255, 90));
        }

        // Rounded indicator arc showing charge
        let arc_center = GPoint::new(x + width as i32 + 20, y + height as i32 / 2);
        let arc_radius = 14;
        draw_arc_aa(canvas, arc_center, arc_radius, -3.14/2.0, -3.14/2.0 + 3.14 * (self.level as f32), Rgb565::from_rgb(0, 200, 255));
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

        context.draw(|canvas: &mut Canvas| {
            // Clear with black background
            canvas.clear_rgb(Rgb565::BLACK);

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

            // Demo AA lines, rounded rect, and alpha blend overlay
            draw_line_rgba_aa(canvas, GPoint::new(10, 120), GPoint::new(150, 160), Rgba8888::new(255, 255, 0, 128));
            fill_rounded_rect(canvas, GRect::new(GPoint::new(180, 100), GSize::new(60, 30)), 8, Rgb565::from_rgb(50, 120, 200));
            fill_rect_rgba(canvas, GRect::new(GPoint::new(190, 110), GSize::new(40, 10)), Rgba8888::new(255, 255, 255, 120));
        }).await;

        context.request_redraw().await;
        Timer::after(Duration::from_millis(16)).await; // 60 FPS
    }
}