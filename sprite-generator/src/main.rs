mod bmp;
mod canvas;

use crate::canvas::Canvas;
use rust_gfx::fluent::*;
/// Sprite Generator - matches main.c output exactly
/// Generates all sprite variations for testing
use rust_gfx::*;
use std::fs;

const SPRITE_WIDTH: usize = 102;
const SPRITE_HEIGHT: usize = 125;

fn main() {
    // Clean up and recreate sprites directory
    if fs::metadata("sprites").is_ok() {
        fs::remove_dir_all("sprites").expect("Failed to remove sprites directory");
    }
    fs::create_dir_all("sprites").expect("Failed to create sprites directory");

    let mut sprite_index = 0;

    println!("Generating individual sprite frames...");

    println!("Generating rectangles...");
    generate_rectangles(&mut sprite_index);

    println!("Generating circles...");
    generate_circles(&mut sprite_index);

    println!("Generating triangles...");
    generate_triangles(&mut sprite_index);

    println!("Generating lines...");
    generate_lines(&mut sprite_index);

    println!("Generating arcs...");
    generate_arcs(&mut sprite_index);

    println!("Generating labels...");
    generate_labels(&mut sprite_index);

    println!("Generating vector graphics...");
    generate_vector_graphics(&mut sprite_index);

    println!("Generating blurs...");
    generate_blurs(&mut sprite_index);

    println!("\nGenerated {} individual sprite frames", sprite_index);
    println!("Saved sprites to: sprites/");
}

fn capture_sprite(canvas: &Canvas, index: &mut usize, name: &str) {
    let filename = format!("sprites/{:04}_{}.bmp", index, name);
    bmp::save_bmp(canvas, &filename).expect("Failed to save BMP");
    *index += 1;
}

fn generate_rectangles(sprite_index: &mut usize) {
    let radii = [0, 5, 10, 20, RADIUS_CIRCLE];
    let radius_names = ["r0", "r5", "r10", "r20", "rcircle"];
    let colors = [
        Rgba8888::rgb(255, 100, 100),
        Rgba8888::rgb(100, 255, 100),
        Rgba8888::rgb(100, 100, 255),
        Rgba8888::rgb(255, 255, 100),
    ];
    let color_names = ["red", "green", "blue", "yellow"];
    let gradient_stops = [
        GradientStop::new(0.0, Rgba8888::rgb(255, 0, 0)),
        GradientStop::new(1.0, Rgba8888::rgb(0, 0, 255)),
    ];
    let gradient_variants = [
        (
            "hor",
            GradientBuilder::linear()
                .axis(Axis::Horizontal)
                .stops(gradient_stops)
                .finish(),
        ),
        (
            "ver",
            GradientBuilder::linear()
                .axis(Axis::Vertical)
                .stops(gradient_stops)
                .finish(),
        ),
        (
            "radial",
            GradientBuilder::radial().stops(gradient_stops).finish(),
        ),
        (
            "conical",
            GradientBuilder::conic().stops(gradient_stops).finish(),
        ),
    ];

    for (radius, radius_name) in radii
        .iter()
        .copied()
        .zip(radius_names.iter().copied())
    {
        for (color, color_name) in colors.iter().copied().zip(color_names.iter().copied()) {
            let mut canvas = Canvas::new(SPRITE_WIDTH, SPRITE_HEIGHT);
            canvas.clear(Rgba8888::TRANSPARENT);

            Rect::new()
                .area(Area::new(20, 30, 82, 95))
                .radius(Radius::uniform(radius))
                .fill(FillPlan::solid(color))
                .finish()
                .draw(&mut canvas);

            let name = format!("rect_solid_{}_{}", radius_name, color_name);
            capture_sprite(&canvas, sprite_index, &name);
        }
    }

    for (grad_name, gradient) in gradient_variants.iter() {
        for (radius, radius_name) in radii
            .iter()
            .copied()
            .take(3)
            .zip(radius_names.iter().copied())
        {
            let mut canvas = Canvas::new(SPRITE_WIDTH, SPRITE_HEIGHT);
            canvas.clear(Rgba8888::TRANSPARENT);

            Rect::new()
                .area(Area::new(20, 30, 82, 95))
                .radius(Radius::uniform(radius))
                .fill(FillPlan::Gradient {
                    gradient: gradient.clone(),
                })
                .finish()
                .draw(&mut canvas);

            let name = format!("rect_grad_{}_{}", grad_name, radius_name);
            capture_sprite(&canvas, sprite_index, &name);
        }
    }

    let grad_border_widths = [3, 6];
    let grad_border_width_names = ["w3", "w6"];
    let grad_border_colors = [Rgba8888::rgb(255, 255, 255), Rgba8888::rgb(40, 40, 40)];
    let grad_border_color_names = ["white", "charcoal"];

    for (grad_name, gradient) in gradient_variants.iter() {
        for (radius, radius_name) in radii
            .iter()
            .copied()
            .take(3)
            .zip(radius_names.iter().copied())
        {
            for ((border_width, border_width_name), (border_color, border_color_name)) in
                grad_border_widths
                    .iter()
                    .copied()
                    .zip(grad_border_width_names.iter().copied())
                    .zip(grad_border_colors.iter().copied().zip(grad_border_color_names.iter().copied()))
            {
                let mut canvas = Canvas::new(SPRITE_WIDTH, SPRITE_HEIGHT);
                canvas.clear(Rgba8888::TRANSPARENT);

                Rect::new()
                    .area(Area::new(20, 30, 82, 95))
                    .radius(Radius::uniform(radius))
                    .fill(FillPlan::Gradient {
                        gradient: gradient.clone(),
                    })
                    .stroke(StrokePlan::solid(border_width, border_color))
                    .finish()
                    .draw(&mut canvas);

                let name = format!(
                    "rect_gradborder_{}_{}_{}_{}",
                    grad_name, radius_name, border_width_name, border_color_name
                );
                capture_sprite(&canvas, sprite_index, &name);
            }
        }
    }

    let border_widths = [1, 3, 6, 10];
    let border_side_pairs = [
        (BorderSide::FULL, "full"),
        (BorderSide::TOP | BorderSide::BOTTOM, "topbottom"),
        (BorderSide::LEFT | BorderSide::RIGHT, "leftright"),
        (BorderSide::TOP, "top"),
    ];

    for border_width in border_widths.iter().copied() {
        for (side, side_name) in border_side_pairs.iter().copied() {
            let mut canvas = Canvas::new(SPRITE_WIDTH, SPRITE_HEIGHT);
            canvas.clear(Rgba8888::TRANSPARENT);

            let stroke = StrokeBuilder::new()
                .width(border_width)
                .color(Rgba8888::rgb(255, 255, 0))
                .sides(side)
                .finish();

            Rect::new()
                .area(Area::new(20, 30, 82, 95))
                .radius(Radius::uniform(10))
                .fill(FillPlan::solid(Rgba8888::rgb(50, 50, 50)))
                .stroke(stroke)
                .finish()
                .draw(&mut canvas);

            let name = format!("rect_border_w{}_{}", border_width, side_name);
            capture_sprite(&canvas, sprite_index, &name);
        }
    }

    let shadow_configs = [[2, 2, 0], [5, 5, 0], [8, 0, 4], [3, 3, 6]];

    for config in shadow_configs { // maintain original order
        for radius in [0, 15] {
            let mut canvas = Canvas::new(SPRITE_WIDTH, SPRITE_HEIGHT);
            canvas.clear(Rgba8888::TRANSPARENT);

            let shadow = ShadowBuilder::new()
                .blur(config[0])
                .offset(Point::new(config[1], config[1]))
                .spread(config[2])
                .color(Rgba8888::rgb(0, 0, 0))
                .opacity(OPA_50)
                .finish();

            Rect::new()
                .area(Area::new(25, 35, 77, 90))
                .radius(Radius::uniform(radius))
                .fill(FillPlan::solid(Rgba8888::rgb(200, 200, 200)))
                .shadow(shadow)
                .finish()
                .draw(&mut canvas);

            let name = format!(
                "rect_shadow_w{}_off{}_spr{}_{}",
                config[0],
                config[1],
                config[2],
                if radius == 0 { "square" } else { "rounded" }
            );
            capture_sprite(&canvas, sprite_index, &name);
        }
    }

    let outline_configs = [[2, 2], [4, 4], [6, 1], [3, 8]];

    for config in outline_configs {
        let mut canvas = Canvas::new(SPRITE_WIDTH, SPRITE_HEIGHT);
        canvas.clear(Rgba8888::TRANSPARENT);

        let outline = OutlineBuilder::with_stroke(StrokePlan::solid(
            config[0],
            Rgba8888::rgb(0, 255, 255),
        ))
        .pad(config[1])
        .finish();

        Rect::new()
            .area(Area::new(30, 40, 72, 85))
            .radius(Radius::uniform(8))
            .fill(FillPlan::solid(Rgba8888::rgb(150, 150, 150)))
            .outline(outline)
            .finish()
            .draw(&mut canvas);

        let name = format!("rect_outline_w{}_pad{}", config[0], config[1]);
        capture_sprite(&canvas, sprite_index, &name);
    }

    let opas = [OPA_COVER, OPA_70, OPA_50, OPA_30];
    let opa_names = ["100", "70", "50", "30"];

    for (opa, opa_name) in opas.iter().copied().zip(opa_names.iter().copied()) {
        let mut canvas = Canvas::new(SPRITE_WIDTH, SPRITE_HEIGHT);
        canvas.clear(Rgba8888::TRANSPARENT);

        let fill = FillBuilder::solid(Rgba8888::rgb(255, 150, 50))
            .opacity(opa)
            .finish();

        Rect::new()
            .area(Area::new(20, 30, 82, 95))
            .radius(Radius::uniform(12))
            .fill(fill)
            .finish()
            .draw(&mut canvas);

        let name = format!("rect_opa{}", opa_name);
        capture_sprite(&canvas, sprite_index, &name);
    }
}

fn generate_circles(sprite_index: &mut usize) {
    let fill_colors = [
        Rgba8888::rgb(255, 120, 120),
        Rgba8888::rgb(120, 255, 180),
        Rgba8888::rgb(120, 180, 255),
        Rgba8888::rgb(255, 220, 120),
    ];
    let fill_names = ["coral", "mint", "sky", "sun"];
    let gradient_stops = [
        GradientStop::new(0.0, Rgba8888::rgb(255, 0, 0)),
        GradientStop::new(1.0, Rgba8888::rgb(0, 0, 255)),
    ];
    let gradient_variants = [
        (
            "hor",
            GradientBuilder::linear()
                .axis(Axis::Horizontal)
                .stops(gradient_stops)
                .finish(),
        ),
        (
            "ver",
            GradientBuilder::linear()
                .axis(Axis::Vertical)
                .stops(gradient_stops)
                .finish(),
        ),
        (
            "radial",
            GradientBuilder::radial().stops(gradient_stops).finish(),
        ),
        (
            "conical",
            GradientBuilder::conic().stops(gradient_stops).finish(),
        ),
    ];
    let area = Area::new(20, 25, 82, 87);

    for (color, name) in fill_colors.iter().copied().zip(fill_names.iter().copied()) {
        let mut canvas = Canvas::new(SPRITE_WIDTH, SPRITE_HEIGHT);
        canvas.clear(Rgba8888::TRANSPARENT);

        Rect::new()
            .area(area)
            .radius(Radius::uniform(RADIUS_CIRCLE))
            .fill(FillPlan::solid(color))
            .finish()
            .draw(&mut canvas);

        let sprite_name = format!("circle_solid_{}", name);
        capture_sprite(&canvas, sprite_index, &sprite_name);
    }

    for (grad_name, gradient) in gradient_variants.iter() {
        let mut canvas = Canvas::new(SPRITE_WIDTH, SPRITE_HEIGHT);
        canvas.clear(Rgba8888::TRANSPARENT);

        Rect::new()
            .area(area)
            .radius(Radius::uniform(RADIUS_CIRCLE))
            .fill(FillPlan::Gradient {
                gradient: gradient.clone(),
            })
            .finish()
            .draw(&mut canvas);

        let sprite_name = format!("circle_grad_{}", grad_name);
        capture_sprite(&canvas, sprite_index, &sprite_name);
    }

    let border_widths = [2, 4, 8];
    let border_width_names = ["w2", "w4", "w8"];
    let border_colors = [
        Rgba8888::rgb(255, 255, 255),
        Rgba8888::rgb(255, 200, 0),
        Rgba8888::rgb(80, 255, 255),
    ];
    let border_color_names = ["white", "gold", "aqua"];

    for (border_width, border_width_name) in border_widths
        .iter()
        .copied()
        .zip(border_width_names.iter().copied())
    {
        for (border_color, border_color_name) in border_colors
            .iter()
            .copied()
            .zip(border_color_names.iter().copied())
        {
            let mut canvas = Canvas::new(SPRITE_WIDTH, SPRITE_HEIGHT);
            canvas.clear(Rgba8888::TRANSPARENT);

            Rect::new()
                .area(area)
                .radius(Radius::uniform(RADIUS_CIRCLE))
                .fill(FillPlan::solid(Rgba8888::rgb(45, 45, 45)))
                .stroke(StrokePlan::solid(border_width, border_color))
                .finish()
                .draw(&mut canvas);

            let sprite_name = format!(
                "circle_border_{}_{}",
                border_width_name, border_color_name
            );
            capture_sprite(&canvas, sprite_index, &sprite_name);
        }
    }

    let accent_gradient_stops = [
        GradientStop::new(0.0, Rgba8888::rgb(255, 80, 0)),
        GradientStop::new(1.0, Rgba8888::rgb(80, 0, 255)),
    ];

    let accent_gradients = [
        (
            "hor",
            GradientBuilder::linear()
                .axis(Axis::Horizontal)
                .stops(accent_gradient_stops)
                .finish(),
        ),
        (
            "ver",
            GradientBuilder::linear()
                .axis(Axis::Vertical)
                .stops(accent_gradient_stops)
                .finish(),
        ),
    ];

    for (grad_name, gradient) in accent_gradients.iter() {
        for (border_width, border_width_name) in border_widths
            .iter()
            .copied()
            .zip(border_width_names.iter().copied())
            .skip(1)
        {
            let mut canvas = Canvas::new(SPRITE_WIDTH, SPRITE_HEIGHT);
            canvas.clear(Rgba8888::TRANSPARENT);

            Rect::new()
                .area(area)
                .radius(Radius::uniform(RADIUS_CIRCLE))
                .fill(FillPlan::Gradient {
                    gradient: gradient.clone(),
                })
                .stroke(StrokePlan::solid(border_width, Rgba8888::rgb(255, 255, 255)))
                .finish()
                .draw(&mut canvas);

            let sprite_name = format!(
                "circle_gradborder_{}_{}",
                grad_name, border_width_name
            );
            capture_sprite(&canvas, sprite_index, &sprite_name);
        }
    }
}

fn generate_triangles(sprite_index: &mut usize) {
    // Different orientations
    let triangles = [
        // Up
        [Point::new(51, 30), Point::new(25, 90), Point::new(77, 90)],
        // Down
        [Point::new(51, 90), Point::new(25, 30), Point::new(77, 30)],
        // Left
        [Point::new(25, 60), Point::new(77, 30), Point::new(77, 90)],
        // Right
        [Point::new(77, 60), Point::new(25, 30), Point::new(25, 90)],
        // Equilateral
        [Point::new(51, 25), Point::new(20, 85), Point::new(82, 85)],
        // Right-angled
        [Point::new(25, 35), Point::new(25, 85), Point::new(75, 85)],
    ];
    let tri_orient_names = ["up", "down", "left", "right", "equi", "rightangle"];

    let tri_colors = [
        Rgba8888::rgb(255, 100, 100),
        Rgba8888::rgb(100, 255, 100),
        Rgba8888::rgb(100, 100, 255),
        Rgba8888::rgb(255, 255, 100),
    ];
    let tri_color_names = ["red", "green", "blue", "yellow"];

    for (t, orient_name) in tri_orient_names.iter().enumerate() {
        for (color, color_name) in tri_colors.iter().copied().zip(tri_color_names.iter().copied()) {
            let mut canvas = Canvas::new(SPRITE_WIDTH, SPRITE_HEIGHT);
            canvas.clear(Rgba8888::TRANSPARENT);

            Triangle::new()
                .vertices(Vertices::new3(triangles[t][0], triangles[t][1], triangles[t][2]))
                .fill(FillPlan::solid(color))
                .finish()
                .draw(&mut canvas);

            let name = format!("tri_{}_{}", orient_name, color_name);
            capture_sprite(&canvas, sprite_index, &name);
        }
    }

    // Gradients
    let tri_gradient_stops = [
        GradientStop::new(0.0, Rgba8888::rgb(255, 0, 255)),
        GradientStop::new(1.0, Rgba8888::rgb(0, 255, 255)),
    ];
    let tri_gradients = [
        (
            "hor",
            GradientBuilder::linear()
                .axis(Axis::Horizontal)
                .stops(tri_gradient_stops)
                .finish(),
        ),
        (
            "ver",
            GradientBuilder::linear()
                .axis(Axis::Vertical)
                .stops(tri_gradient_stops)
                .finish(),
        ),
    ];

    for (grad_name, gradient) in tri_gradients.iter() {
        for (t, orient_name) in tri_orient_names.iter().take(3).enumerate() {
            let mut canvas = Canvas::new(SPRITE_WIDTH, SPRITE_HEIGHT);
            canvas.clear(Rgba8888::TRANSPARENT);

            Triangle::new()
                .vertices(Vertices::new3(triangles[t][0], triangles[t][1], triangles[t][2]))
                .fill(FillPlan::Gradient {
                    gradient: gradient.clone(),
                })
                .finish()
                .draw(&mut canvas);

            let name = format!("tri_grad_{}_{}", grad_name, orient_name);
            capture_sprite(&canvas, sprite_index, &name);
        }
    }

    // Opacity variations
    let opas = [OPA_COVER, OPA_70, OPA_40];
    let opa_names = ["100", "70", "40"];

    for (opa, opa_name) in opas.iter().copied().zip(opa_names.iter().copied()) {
        let mut canvas = Canvas::new(SPRITE_WIDTH, SPRITE_HEIGHT);
        canvas.clear(Rgba8888::TRANSPARENT);

        let fill = FillBuilder::solid(Rgba8888::rgb(255, 128, 0))
            .opacity(opa)
            .finish();

        Triangle::new()
            .vertices(Vertices::new3(triangles[0][0], triangles[0][1], triangles[0][2]))
            .fill(fill)
            .finish()
            .draw(&mut canvas);

        let name = format!("tri_opa{}", opa_name);
        capture_sprite(&canvas, sprite_index, &name);
    }

    let tri_border_indices = [0, 1, 4];
    let border_widths = [2, 5];
    let border_width_names = ["w2", "w5"];
    let border_colors = [
        Rgba8888::rgb(255, 255, 255),
        Rgba8888::rgb(255, 220, 0),
        Rgba8888::rgb(255, 105, 180),
    ];
    let border_color_names = ["white", "gold", "pink"];

    for (ti, tri_idx) in tri_border_indices.iter().copied().enumerate() {
        for (border_width, border_width_name) in border_widths
            .iter()
            .copied()
            .zip(border_width_names.iter().copied())
        {
            for (color_idx, (border_color, border_color_name)) in border_colors
                .iter()
                .copied()
                .zip(border_color_names.iter().copied())
                .enumerate()
            {
                let mut canvas = Canvas::new(SPRITE_WIDTH, SPRITE_HEIGHT);
                canvas.clear(Rgba8888::TRANSPARENT);

                let fill_color = tri_colors[(ti + color_idx) % tri_colors.len()];
                let stroke = StrokePlan::solid(border_width, border_color);

                Triangle::new()
                    .vertices(Vertices::new3(triangles[tri_idx][0], triangles[tri_idx][1], triangles[tri_idx][2]))
                    .fill(FillPlan::solid(fill_color))
                    .stroke(stroke)
                    .finish()
                    .draw(&mut canvas);

                let name = format!(
                    "tri_border_{}_{}_{}",
                    tri_orient_names[tri_idx], border_width_name, border_color_name
                );
                capture_sprite(&canvas, sprite_index, &name);
            }
        }
    }
}

fn generate_lines(sprite_index: &mut usize) {
    let widths = [1, 3, 6, 10];

    // Horizontal
    for width in widths.iter().copied() {
        let mut canvas = Canvas::new(SPRITE_WIDTH, SPRITE_HEIGHT);
        canvas.clear(Rgba8888::TRANSPARENT);

        Line::new()
            .vertices(Vertices::new(Point::new(15, 62), Point::new(87, 62)))
            .stroke(StrokePlan::solid(width, Rgba8888::rgb(255, 255, 255)))
            .finish()
            .draw(&mut canvas);

        let name = format!("line_hor_w{}", width);
        capture_sprite(&canvas, sprite_index, &name);
    }

    // Vertical
    for width in widths.iter().copied() {
        let mut canvas = Canvas::new(SPRITE_WIDTH, SPRITE_HEIGHT);
        canvas.clear(Rgba8888::TRANSPARENT);

        Line::new()
            .vertices(Vertices::new(Point::new(51, 25), Point::new(51, 100)))
            .stroke(StrokePlan::solid(width, Rgba8888::rgb(255, 255, 0)))
            .finish()
            .draw(&mut canvas);

        let name = format!("line_ver_w{}", width);
        capture_sprite(&canvas, sprite_index, &name);
    }

    // Diagonal
    for width in widths.iter().copied() {
        let mut canvas = Canvas::new(SPRITE_WIDTH, SPRITE_HEIGHT);
        canvas.clear(Rgba8888::TRANSPARENT);

        Line::new()
            .vertices(Vertices::new(Point::new(20, 30), Point::new(82, 95)))
            .stroke(StrokePlan::solid(width, Rgba8888::rgb(0, 255, 255)))
            .finish()
            .draw(&mut canvas);

        let name = format!("line_diag_w{}", width);
        capture_sprite(&canvas, sprite_index, &name);
    }

    // Dashed patterns
    let dash_configs = [[5, 3], [10, 5], [2, 2], [8, 2]];

    for config in dash_configs {
        let mut canvas = Canvas::new(SPRITE_WIDTH, SPRITE_HEIGHT);
        canvas.clear(Rgba8888::TRANSPARENT);

        let stroke = StrokePlan::solid(3, Rgba8888::rgb(255, 100, 255));
        let dash = DashPattern::new(&[
            DashUnit::pixels(config[0]),
            DashUnit::pixels(config[1]),
        ]);

        Line::new()
            .vertices(Vertices::new(Point::new(15, 62), Point::new(87, 62)))
            .stroke(stroke)
            .dash(dash)
            .finish()
            .draw(&mut canvas);

        let name = format!("line_dash_w{}_g{}", config[0], config[1]);
        capture_sprite(&canvas, sprite_index, &name);
    }

    // Round caps
    let cap_configs = [[false, false], [true, false], [false, true], [true, true]];
    let cap_names = ["none", "start", "end", "both"];

    for (idx, caps) in cap_configs.iter().enumerate() {
        let mut canvas = Canvas::new(SPRITE_WIDTH, SPRITE_HEIGHT);
        canvas.clear(Rgba8888::TRANSPARENT);

        let caps = LineCaps::with(
            if caps[0] { LineCap::Round } else { LineCap::Butt },
            if caps[1] { LineCap::Round } else { LineCap::Butt },
        );

        Line::new()
            .vertices(Vertices::new(Point::new(20, 40), Point::new(82, 85)))
            .stroke(StrokePlan::solid(8, Rgba8888::rgb(100, 255, 100)))
            .caps(caps)
            .finish()
            .draw(&mut canvas);

        let name = format!("line_cap_{}", cap_names[idx]);
        capture_sprite(&canvas, sprite_index, &name);
    }

    // Opacity
    let opas = [OPA_COVER, OPA_70, OPA_40];
    let opa_names = ["100", "70", "40"];

    for (opa, opa_name) in opas.iter().copied().zip(opa_names.iter().copied()) {
        let mut canvas = Canvas::new(SPRITE_WIDTH, SPRITE_HEIGHT);
        canvas.clear(Rgba8888::TRANSPARENT);

        let stroke = StrokeBuilder::new()
            .width(5)
            .color(Rgba8888::rgb(255, 50, 50))
            .opacity(opa)
            .finish();

        Line::new()
            .vertices(Vertices::new(Point::new(15, 62), Point::new(87, 62)))
            .stroke(stroke)
            .finish()
            .draw(&mut canvas);

        let name = format!("line_opa{}", opa_name);
        capture_sprite(&canvas, sprite_index, &name);
    }
}

fn generate_arcs(sprite_index: &mut usize) {
    let widths = [3, 8, 15];

    // Quarter arcs at different positions
    let start_angles = [0, 90, 180, 270];

    for width in widths.iter().copied() {
        for start in start_angles {
            let mut canvas = Canvas::new(SPRITE_WIDTH, SPRITE_HEIGHT);
            canvas.clear(Rgba8888::TRANSPARENT);

            Arc::new()
                .center(Point::new(51, 62))
                .radius(Radius::uniform(35))
                .angles(Angles::new(
                    Angle::from_degrees(start),
                    Angle::from_degrees(start + 90),
                ))
                .stroke(StrokePlan::solid(width, Rgba8888::rgb(255, 200, 0)))
                .finish()
                .draw(&mut canvas);

            let name = format!("arc_quarter_w{}_a{}", width, start);
            capture_sprite(&canvas, sprite_index, &name);
        }
    }

    // Different arc spans
    let arc_spans = [45, 90, 180, 270];

    for span in arc_spans {
        let mut canvas = Canvas::new(SPRITE_WIDTH, SPRITE_HEIGHT);
        canvas.clear(Rgba8888::TRANSPARENT);

        Arc::new()
            .center(Point::new(51, 62))
            .radius(Radius::uniform(35))
            .angles(Angles::new(Angle::from_degrees(0), Angle::from_degrees(span)))
            .stroke(StrokePlan::solid(8, Rgba8888::rgb(100, 255, 255)))
            .finish()
            .draw(&mut canvas);

        let name = format!("arc_span{}", span);
        capture_sprite(&canvas, sprite_index, &name);
    }

    // Rounded ends
    for width in widths.iter().copied() {
        let mut canvas = Canvas::new(SPRITE_WIDTH, SPRITE_HEIGHT);
        canvas.clear(Rgba8888::TRANSPARENT);

        Arc::new()
            .center(Point::new(51, 62))
            .radius(Radius::uniform(35))
            .angles(Angles::new(Angle::from_degrees(45), Angle::from_degrees(225)))
            .stroke(StrokePlan::solid(width, Rgba8888::rgb(255, 100, 255)))
            .rounded(true)
            .finish()
            .draw(&mut canvas);

        let name = format!("arc_rounded_w{}", width);
        capture_sprite(&canvas, sprite_index, &name);
    }

    // Opacity variations
    let opas = [OPA_COVER, OPA_70, OPA_40];
    let opa_names = ["100", "70", "40"];

    for (opa, opa_name) in opas.iter().copied().zip(opa_names.iter().copied()) {
        let mut canvas = Canvas::new(SPRITE_WIDTH, SPRITE_HEIGHT);
        canvas.clear(Rgba8888::TRANSPARENT);

        let stroke = StrokeBuilder::new()
            .width(10)
            .color(Rgba8888::rgb(255, 50, 50))
            .opacity(opa)
            .finish();

        Arc::new()
            .center(Point::new(51, 62))
            .radius(Radius::uniform(35))
            .angles(Angles::new(Angle::from_degrees(0), Angle::from_degrees(270)))
            .stroke(stroke)
            .finish()
            .draw(&mut canvas);

        let name = format!("arc_opa{}", opa_name);
        capture_sprite(&canvas, sprite_index, &name);
    }

    // Different colors
    let colors = [
        Rgba8888::rgb(255, 0, 0),
        Rgba8888::rgb(0, 255, 0),
        Rgba8888::rgb(0, 0, 255),
        Rgba8888::rgb(255, 255, 0),
    ];
    let color_names = ["red", "green", "blue", "yellow"];

    for (color, name) in colors.iter().copied().zip(color_names.iter().copied()) {
        let mut canvas = Canvas::new(SPRITE_WIDTH, SPRITE_HEIGHT);
        canvas.clear(Rgba8888::TRANSPARENT);

        Arc::new()
            .center(Point::new(51, 62))
            .radius(Radius::uniform(35))
            .angles(Angles::new(Angle::from_degrees(0), Angle::from_degrees(180)))
            .stroke(StrokePlan::solid(6, color))
            .finish()
            .draw(&mut canvas);

        let sprite_name = format!("arc_color_{}", name);
        capture_sprite(&canvas, sprite_index, &sprite_name);
    }
}

fn generate_labels(sprite_index: &mut usize) {
    let texts = ["A", "AB", "ABC", "Text", "123", "!@#"];
    let text_names = ["A", "AB", "ABC", "Text", "123", "sym"];
    let origin = Point::new(30, 50);

    for (text, name) in texts.iter().zip(text_names.iter()) {
        let mut canvas = Canvas::new(SPRITE_WIDTH, SPRITE_HEIGHT);
        canvas.clear(Rgba8888::TRANSPARENT);

        Label::new()
            .origin(origin)
            .content(LabelContentPlan::text(
                FontHandle::named("montserrat", 14),
                *text,
            ))
            .color(Rgba8888::rgb(255, 255, 255))
            .finish()
            .draw(&mut canvas);

        let sprite_name = format!("label_text_{}", name);
        capture_sprite(&canvas, sprite_index, &sprite_name);
    }

    let decors = [
        LabelDecor::None,
        LabelDecor::Underline,
        LabelDecor::Strikethrough,
    ];
    let decor_names = ["none", "underline", "strike"];

    for (decor, name) in decors.iter().zip(decor_names.iter()) {
        let mut canvas = Canvas::new(SPRITE_WIDTH, SPRITE_HEIGHT);
        canvas.clear(Rgba8888::TRANSPARENT);

        Label::new()
            .origin(Point::new(25, 50))
            .content(LabelContentPlan::text(
                FontHandle::named("montserrat", 14),
                "Test",
            ))
            .color(Rgba8888::rgb(255, 255, 0))
            .decor(*decor)
            .finish()
            .draw(&mut canvas);

        let sprite_name = format!("label_decor_{}", name);
        capture_sprite(&canvas, sprite_index, &sprite_name);
    }

    let spacings = [0, 5, 10];

    for spacing in spacings.iter().copied() {
        let mut canvas = Canvas::new(SPRITE_WIDTH, SPRITE_HEIGHT);
        canvas.clear(Rgba8888::TRANSPARENT);

        Label::new()
            .origin(Point::new(15, 50))
            .content(LabelContentPlan::text(
                FontHandle::named("montserrat", 14),
                "Abc",
            ))
            .color(Rgba8888::rgb(100, 255, 255))
            .spacing(LabelSpacing::new(spacing, 0))
            .finish()
            .draw(&mut canvas);

        let sprite_name = format!("label_spacing{}", spacing);
        capture_sprite(&canvas, sprite_index, &sprite_name);
    }

    let opas = [OPA_COVER, OPA_70, OPA_40];
    let opa_names = ["100", "70", "40"];

    for (opa, name) in opas.iter().copied().zip(opa_names.iter().copied()) {
        let mut canvas = Canvas::new(SPRITE_WIDTH, SPRITE_HEIGHT);
        canvas.clear(Rgba8888::TRANSPARENT);

        Label::new()
            .origin(Point::new(25, 50))
            .content(LabelContentPlan::text(
                FontHandle::named("montserrat", 14),
                "Text",
            ))
            .color(Rgba8888::rgb(255, 100, 255))
            .opacity(LabelOpacity::new(opa))
            .finish()
            .draw(&mut canvas);

        let sprite_name = format!("label_opa{}", name);
        capture_sprite(&canvas, sprite_index, &sprite_name);
    }

    let colors = [
        Rgba8888::rgb(255, 0, 0),
        Rgba8888::rgb(0, 255, 0),
        Rgba8888::rgb(0, 0, 255),
        Rgba8888::rgb(255, 128, 0),
    ];
    let color_names = ["red", "green", "blue", "orange"];

    for (color, name) in colors.iter().copied().zip(color_names.iter().copied()) {
        let mut canvas = Canvas::new(SPRITE_WIDTH, SPRITE_HEIGHT);
        canvas.clear(Rgba8888::TRANSPARENT);

        Label::new()
            .origin(Point::new(30, 50))
            .content(LabelContentPlan::text(
                FontHandle::named("montserrat", 14),
                "123",
            ))
            .color(color)
            .finish()
            .draw(&mut canvas);

        let sprite_name = format!("label_color_{}", name);
        capture_sprite(&canvas, sprite_index, &sprite_name);
    }
}

fn generate_vector_graphics(sprite_index: &mut usize) {
    let mut canvas = Canvas::new(SPRITE_WIDTH, SPRITE_HEIGHT);
    canvas.clear(Rgba8888::TRANSPARENT);

    let star_path = PathPlan::from_svg(
        "M 51 25 L 62 45 L 85 48 L 66 62 L 74 86 L 51 72 L 28 86 L 36 62 L 17 48 L 40 45 Z",
    );
    let star_gradient = GradientBuilder::linear()
        .axis(Axis::Horizontal)
        .start_point(Point::new(25, 30))
        .end_point(Point::new(80, 95))
        .stops([
            GradientStop::new(0.0, Rgba8888::rgb(255, 90, 0)),
            GradientStop::new(130.0 / 255.0, Rgba8888::rgb(255, 0, 200)),
            GradientStop::new(1.0, Rgba8888::rgb(80, 200, 255)),
        ])
        .finish();
    let star_stroke = StrokeBuilder::new()
        .width(3)
        .color(Rgba8888::rgb(255, 255, 255))
        .opacity(OPA_70)
        .caps(LineCaps::round())
        .join(StrokeJoinStyle::Round)
        .finish();

    Vector::new()
        .path(star_path)
        .fill(FillPlan::Gradient {
            gradient: star_gradient,
        })
        .stroke(star_stroke)
        .finish()
        .draw(&mut canvas);

    capture_sprite(&canvas, sprite_index, "vector_star_gradient");

    let mut canvas = Canvas::new(SPRITE_WIDTH, SPRITE_HEIGHT);
    canvas.clear(Rgba8888::TRANSPARENT);

    let wave_path = PathPlan::from_svg(
        "M 22 82 C 35 35 65 95 84 44 C 70 24 40 24 26 46",
    );
    let wave_gradient = GradientBuilder::linear()
        .axis(Axis::Horizontal)
        .start_point(Point::new(22, 82))
        .end_point(Point::new(84, 44))
        .stops([
            GradientStop::new(0.0, Rgba8888::rgb(120, 255, 120)),
            GradientStop::new(1.0, Rgba8888::rgb(0, 150, 255)),
        ])
        .finish();
    let wave_stroke = StrokeBuilder::new()
        .width(6)
        .gradient(wave_gradient)
        .caps(LineCaps::round())
        .join(StrokeJoinStyle::Round)
        .dash_pattern(&[14.0, 6.0])
        .finish();

    Vector::new()
        .path(wave_path)
        .stroke(wave_stroke)
        .finish()
        .draw(&mut canvas);

    capture_sprite(&canvas, sprite_index, "vector_wave_stroke");
}

fn generate_blurs(sprite_index: &mut usize) {
    let blur_radii = [2, 5, 10, 15];
    let corner_radii = [0, 10, 20];
    let corner_names = ["square", "r10", "r20"];

    let gradient = GradientBuilder::linear()
        .axis(Axis::Horizontal)
        .stops([
            GradientStop::new(0.0, Rgba8888::rgb(255, 0, 0)),
            GradientStop::new(1.0, Rgba8888::rgb(0, 0, 255)),
        ])
        .finish();

    for blur_radius in blur_radii.iter().copied() {
        for (corner_radius, corner_name) in corner_radii
            .iter()
            .copied()
            .zip(corner_names.iter().copied())
        {
            let mut canvas = Canvas::new(SPRITE_WIDTH, SPRITE_HEIGHT);
            canvas.clear(Rgba8888::TRANSPARENT);

            let area = Area::new(20, 30, 82, 95);

            Rect::new()
                .area(area)
                .radius(Radius::uniform(corner_radius))
                .fill(FillPlan::Gradient {
                    gradient: gradient.clone(),
                })
                .finish()
                .draw(&mut canvas);

            Blur::new()
                .area(area)
                .radius(blur_radius)
                .corner_radius(corner_radius)
                .finish()
                .draw(&mut canvas);

            let name = format!("blur_r{}_{}", blur_radius, corner_name);
            capture_sprite(&canvas, sprite_index, &name);
        }
    }
}
