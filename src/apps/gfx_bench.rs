#![no_std]

extern crate alloc;
use rust_gfx::color::Rgba8888;
use rust_gfx::primitives::*;
use rust_gfx::rasterizer::Rasterizer;
use rust_gfx::{Area, BorderSide, GradDir, GradStop, Gradient, OPA_30, OPA_50, OPA_70, OPA_COVER, RADIUS_CIRCLE};
use crate::system::app::app_context::AppContext;
use crate::system::input::types::{HighLevelEvent, TouchAction};
use crate::system::ui::drawing_surface::DrawingSurface;
use alloc::vec::Vec;
use defmt::info;
use embassy_time::{Duration, Instant, Timer};

#[derive(Debug, Clone, Copy)]
struct SpriteTest {
    name: &'static str,
    radius: i32,
    bg_color: Rgba8888,
    border_width: i32,
    border_color: Option<Rgba8888>,
    bg_opa: u8,
    grad_dir: GradDir,
    grad_color2: Option<Rgba8888>,
}

fn generate_working_sprites() -> Vec<SpriteTest> {
    let mut tests = Vec::new();

    let radii = [0, 5, 10, 20, RADIUS_CIRCLE];
    let radius_names = ["r0", "r5", "r10", "r20", "circle"];
    let colors = [
        Rgba8888::rgb(255, 100, 100),
        Rgba8888::rgb(100, 255, 100),
        Rgba8888::rgb(100, 100, 255),
        Rgba8888::rgb(255, 255, 100),
    ];
    let color_names = ["red", "green", "blue", "yellow"];

    // 20 solid fills with various radius (sprites 0-19)
    for r in 0..5 {
        for c in 0..4 {
            tests.push(SpriteTest {
                name: match (radius_names[r], color_names[c]) {
                    ("r0", "red") => "rect_solid_r0_red",
                    ("r0", "green") => "rect_solid_r0_green",
                    ("r0", "blue") => "rect_solid_r0_blue",
                    ("r0", "yellow") => "rect_solid_r0_yellow",
                    ("r5", "red") => "rect_solid_r5_red",
                    ("r5", "green") => "rect_solid_r5_green",
                    ("r5", "blue") => "rect_solid_r5_blue",
                    ("r5", "yellow") => "rect_solid_r5_yellow",
                    ("r10", "red") => "rect_solid_r10_red",
                    ("r10", "green") => "rect_solid_r10_green",
                    ("r10", "blue") => "rect_solid_r10_blue",
                    ("r10", "yellow") => "rect_solid_r10_yellow",
                    ("r20", "red") => "rect_solid_r20_red",
                    ("r20", "green") => "rect_solid_r20_green",
                    ("r20", "blue") => "rect_solid_r20_blue",
                    ("r20", "yellow") => "rect_solid_r20_yellow",
                    ("circle", "red") => "rect_solid_rcircle_red",
                    ("circle", "green") => "rect_solid_rcircle_green",
                    ("circle", "blue") => "rect_solid_rcircle_blue",
                    ("circle", "yellow") => "rect_solid_rcircle_yellow",
                    _ => "unknown",
                },
                radius: radii[r],
                bg_color: colors[c],
                border_width: 0,
                border_color: None,
                bg_opa: OPA_COVER,
                grad_dir: GradDir::None,
                grad_color2: None,
            });
        }
    }

    // Horizontal gradients (sprites 20-22)
    let grad_radii = [0, 5, 10];
    let grad_names = ["hor_r0", "hor_r5", "hor_r10"];
    for (i, &radius) in grad_radii.iter().enumerate() {
        tests.push(SpriteTest {
            name: match grad_names[i] {
                "hor_r0" => "rect_grad_hor_r0",
                "hor_r5" => "rect_grad_hor_r5",
                "hor_r10" => "rect_grad_hor_r10",
                _ => "unknown",
            },
            radius,
            bg_color: Rgba8888::rgb(255, 0, 0), // Start color: red
            border_width: 0,
            border_color: None,
            bg_opa: OPA_COVER,
            grad_dir: GradDir::Hor,
            grad_color2: Some(Rgba8888::rgb(0, 0, 255)), // End color: blue
        });
    }

    // Vertical gradients (sprites 23-25)
    let grad_names_v = ["ver_r0", "ver_r5", "ver_r10"];
    for (i, &radius) in grad_radii.iter().enumerate() {
        tests.push(SpriteTest {
            name: match grad_names_v[i] {
                "ver_r0" => "rect_grad_ver_r0",
                "ver_r5" => "rect_grad_ver_r5",
                "ver_r10" => "rect_grad_ver_r10",
                _ => "unknown",
            },
            radius,
            bg_color: Rgba8888::rgb(255, 0, 0), // Start color: red
            border_width: 0,
            border_color: None,
            bg_opa: OPA_COVER,
            grad_dir: GradDir::Ver,
            grad_color2: Some(Rgba8888::rgb(0, 0, 255)), // End color: blue
        });
    }

    // Border w10 full (sprite 44)
    tests.push(SpriteTest {
        name: "rect_border_w10_full",
        radius: 10,
        bg_color: Rgba8888::rgb(50, 50, 50),
        border_width: 10,
        border_color: Some(Rgba8888::rgb(255, 255, 0)),
        bg_opa: OPA_COVER,
        grad_dir: GradDir::None,
        grad_color2: None,
    });

    // Opacity variations (sprites 60-63)
    tests.push(SpriteTest {
        name: "rect_opa100",
        radius: 12,
        bg_color: Rgba8888::rgb(255, 150, 50),
        border_width: 0,
        border_color: None,
        bg_opa: OPA_COVER,
        grad_dir: GradDir::None,
        grad_color2: None,
    });

    tests.push(SpriteTest {
        name: "rect_opa70",
        radius: 12,
        bg_color: Rgba8888::rgb(255, 150, 50),
        border_width: 0,
        border_color: None,
        bg_opa: OPA_70,
        grad_dir: GradDir::None,
        grad_color2: None,
    });

    tests.push(SpriteTest {
        name: "rect_opa50",
        radius: 12,
        bg_color: Rgba8888::rgb(255, 150, 50),
        border_width: 0,
        border_color: None,
        bg_opa: OPA_50,
        grad_dir: GradDir::None,
        grad_color2: None,
    });

    tests.push(SpriteTest {
        name: "rect_opa30",
        radius: 12,
        bg_color: Rgba8888::rgb(255, 150, 50),
        border_width: 0,
        border_color: None,
        bg_opa: OPA_30,
        grad_dir: GradDir::None,
        grad_color2: None,
    });

    tests
}

fn execute_test(surface: &mut DrawingSurface, test: SpriteTest) {
    let w = surface.width() as i32;
    let h = surface.height() as i32;

    // Scale to fit screen - use same proportions as sprite_generator
    let sprite_w = 62; // Area width from sprite generator: 82-20
    let sprite_h = 65; // Area height: 95-30
    
    // Center the shape
    let x1 = (w - sprite_w) / 2;
    let y1 = (h - sprite_h) / 2;
    let x2 = x1 + sprite_w;
    let y2 = y1 + sprite_h;

    // Create descriptor matching sprite generator
    let mut dsc = RectDsc::new();
    dsc.radius = test.radius;
    dsc.bg_opa = test.bg_opa;
    dsc.bg_color = test.bg_color;

    // Add gradient if specified
    if let Some(color2) = test.grad_color2 {
        dsc.bg_grad = Gradient {
            dir: test.grad_dir,
            stops: [
                GradStop {
                    color: test.bg_color,
                    opa: OPA_COVER,
                    frac: 0,
                },
                GradStop {
                    color: color2,
                    opa: OPA_COVER,
                    frac: 255,
                },
            ],
            stops_count: 2,
        };
    }

    if let Some(border_color) = test.border_color {
        dsc.border_opa = OPA_COVER;
        dsc.border_width = test.border_width;
        dsc.border_color = border_color;
        dsc.border_side = BorderSide::FULL;
    }

    let area = Area::new(x1, y1, x2, y2);
    
    // Draw directly to surface using rust-gfx primitive
    draw_rect(surface, &dsc, &area);
}

#[embassy_executor::task]
pub async fn gfx_bench_app(context: AppContext) {
    info!("Starting gfx_bench - showcasing 31 working sprites");

    let tests = generate_working_sprites();
    info!("Generated {} test configurations", tests.len());

    let mut current_test = 0;

    loop {
        if !context.is_focused().await {
            Timer::after(Duration::from_millis(100)).await;
            continue;
        }

        let test = tests[current_test];
        
        let draw_start = Instant::now();
        context
            .draw(|surface: &mut DrawingSurface| {
                // Clear to black background
                surface.fill_rect(
                    0,
                    0,
                    surface.width() as i32,
                    surface.height() as i32,
                    Rgba8888::rgba(0, 0, 0, 255),
                );
                
                // Draw the test sprite
                execute_test(surface, test);
            })
            .await;
        let elapsed = draw_start.elapsed();

        info!(
            "Test {}/{}: {} - rendered in {} us",
            current_test + 1,
            tests.len(),
            test.name,
            elapsed.as_micros()
        );

        // Wait for touch to proceed to next test
        loop {
            if let Some(HighLevelEvent::Motion(motion)) = context.poll_input().await {
                if matches!(motion.action, TouchAction::Down) {
                    current_test = (current_test + 1) % tests.len();
                    break;
                }
            }

            Timer::after(Duration::from_millis(10)).await;
        }
    }
}
