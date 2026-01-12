/// Sprite Generator - matches main.c output exactly
/// Generates all sprite variations for testing
use rust_gfx::*;
use rust_gfx::primitives::*;
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

    println!("Generating triangles...");
    generate_triangles(&mut sprite_index);

    println!("Generating lines...");
    generate_lines(&mut sprite_index);

    println!("Generating arcs...");
    generate_arcs(&mut sprite_index);

    println!("Generating labels...");
    generate_labels(&mut sprite_index);

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

    // Solid fills with various radius
    for r in 0..5 {
        for c in 0..4 {
            let mut canvas = Canvas::new(SPRITE_WIDTH, SPRITE_HEIGHT);
            canvas.clear(Rgba8888::TRANSPARENT);

            let mut dsc = RectDsc::new();
            dsc.radius = radii[r];
            dsc.bg_opa = OPA_COVER;
            dsc.bg_color = colors[c];

            let area = Area::new(20, 30, 82, 95);
            draw_rect(&mut canvas, &dsc, &area);

            let name = format!("rect_solid_{}_{}", radius_names[r], color_names[c]);
            capture_sprite(&canvas, sprite_index, &name);
        }
    }

    // Gradients (horizontal, vertical, radial, conical)
    let grad_dirs = [GradDir::Hor, GradDir::Ver, GradDir::Radial, GradDir::Conical];
    let grad_names = ["hor", "ver", "radial", "conical"];

    for g in 0..4 {
        for r in 0..3 {
            let mut canvas = Canvas::new(SPRITE_WIDTH, SPRITE_HEIGHT);
            canvas.clear(Rgba8888::TRANSPARENT);

            let mut dsc = RectDsc::new();
            dsc.radius = radii[r];
            dsc.bg_opa = OPA_COVER;
            dsc.bg_color = Rgba8888::rgb(255, 0, 0);
            dsc.bg_grad = Gradient {
                dir: grad_dirs[g],
                stops: [
                    GradStop {
                        color: Rgba8888::rgb(255, 0, 0),
                        opa: OPA_COVER,
                        frac: 0,
                    },
                    GradStop {
                        color: Rgba8888::rgb(0, 0, 255),
                        opa: OPA_COVER,
                        frac: 255,
                    },
                ],
                stops_count: 2,
            };

            let area = Area::new(20, 30, 82, 95);
            draw_rect(&mut canvas, &dsc, &area);

            let name = format!("rect_grad_{}_{}", grad_names[g], radius_names[r]);
            capture_sprite(&canvas, sprite_index, &name);
        }
    }

    // Borders with various widths and sides
    let border_widths = [1, 3, 6, 10];
    let border_sides = [
        BorderSide::FULL,
        BorderSide::TOP | BorderSide::BOTTOM,
        BorderSide::LEFT | BorderSide::RIGHT,
        BorderSide::TOP,
    ];
    let border_side_names = ["full", "topbottom", "leftright", "top"];

    for w in 0..4 {
        for s in 0..4 {
            let mut canvas = Canvas::new(SPRITE_WIDTH, SPRITE_HEIGHT);
            canvas.clear(Rgba8888::TRANSPARENT);

            let mut dsc = RectDsc::new();
            dsc.radius = 10;
            dsc.bg_opa = OPA_COVER;
            dsc.bg_color = Rgba8888::rgb(50, 50, 50);
            dsc.border_opa = OPA_COVER;
            dsc.border_width = border_widths[w];
            dsc.border_color = Rgba8888::rgb(255, 255, 0);
            dsc.border_side = border_sides[s];

            let area = Area::new(20, 30, 82, 95);
            draw_rect(&mut canvas, &dsc, &area);

            let name = format!("rect_border_w{}_{}", border_widths[w], border_side_names[s]);
            capture_sprite(&canvas, sprite_index, &name);
        }
    }

    // Shadows
    let shadow_configs = [[2, 2, 0], [5, 5, 0], [8, 0, 4], [3, 3, 6]];

    for i in 0..4 {
        for r in 0..2 {
            let mut canvas = Canvas::new(SPRITE_WIDTH, SPRITE_HEIGHT);
            canvas.clear(Rgba8888::TRANSPARENT);

            let mut dsc = RectDsc::new();
            dsc.radius = if r == 0 { 0 } else { 15 };
            dsc.bg_opa = OPA_COVER;
            dsc.bg_color = Rgba8888::rgb(200, 200, 200);
            dsc.shadow_opa = OPA_50;
            dsc.shadow_width = shadow_configs[i][0];
            dsc.shadow_offset_x = shadow_configs[i][1];
            dsc.shadow_offset_y = shadow_configs[i][1];
            dsc.shadow_spread = shadow_configs[i][2];
            dsc.shadow_color = Rgba8888::rgb(0, 0, 0);

            let area = Area::new(25, 35, 77, 90);
            draw_rect(&mut canvas, &dsc, &area);

            let name = format!(
                "rect_shadow_w{}_off{}_spr{}_{}",
                shadow_configs[i][0],
                shadow_configs[i][1],
                shadow_configs[i][2],
                if r == 0 { "square" } else { "rounded" }
            );
            capture_sprite(&canvas, sprite_index, &name);
        }
    }

    // Outlines
    let outline_configs = [[2, 2], [4, 4], [6, 1], [3, 8]];

    for i in 0..4 {
        let mut canvas = Canvas::new(SPRITE_WIDTH, SPRITE_HEIGHT);
        canvas.clear(Rgba8888::TRANSPARENT);

        let mut dsc = RectDsc::new();
        dsc.radius = 8;
        dsc.bg_opa = OPA_COVER;
        dsc.bg_color = Rgba8888::rgb(150, 150, 150);
        dsc.outline_opa = OPA_COVER;
        dsc.outline_width = outline_configs[i][0];
        dsc.outline_pad = outline_configs[i][1];
        dsc.outline_color = Rgba8888::rgb(0, 255, 255);

        let area = Area::new(30, 40, 72, 85);
        draw_rect(&mut canvas, &dsc, &area);

        let name = format!("rect_outline_w{}_pad{}", outline_configs[i][0], outline_configs[i][1]);
        capture_sprite(&canvas, sprite_index, &name);
    }

    // Opacity variations
    let opas = [OPA_COVER, OPA_70, OPA_50, OPA_30];
    let opa_names = ["100", "70", "50", "30"];

    for o in 0..4 {
        let mut canvas = Canvas::new(SPRITE_WIDTH, SPRITE_HEIGHT);
        canvas.clear(Rgba8888::TRANSPARENT);

        let mut dsc = RectDsc::new();
        dsc.radius = 12;
        dsc.bg_opa = opas[o];
        dsc.bg_color = Rgba8888::rgb(255, 150, 50);

        let area = Area::new(20, 30, 82, 95);
        draw_rect(&mut canvas, &dsc, &area);

        let name = format!("rect_opa{}", opa_names[o]);
        capture_sprite(&canvas, sprite_index, &name);
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

    for t in 0..6 {
        for c in 0..4 {
            let mut canvas = Canvas::new(SPRITE_WIDTH, SPRITE_HEIGHT);
            canvas.clear(Rgba8888::TRANSPARENT);

            let dsc = TriangleDsc {
                p1: triangles[t][0],
                p2: triangles[t][1],
                p3: triangles[t][2],
                color: tri_colors[c],
                opa: OPA_COVER,
                grad: Gradient::none(),
            };

            draw_triangle(&mut canvas, &dsc);

            let name = format!("tri_{}_{}", tri_orient_names[t], tri_color_names[c]);
            capture_sprite(&canvas, sprite_index, &name);
        }
    }

    // Gradients
    let grad_dirs = [GradDir::Hor, GradDir::Ver];
    let grad_names = ["hor", "ver"];

    for g in 0..2 {
        for t in 0..3 {
            let mut canvas = Canvas::new(SPRITE_WIDTH, SPRITE_HEIGHT);
            canvas.clear(Rgba8888::TRANSPARENT);

            let dsc = TriangleDsc {
                p1: triangles[t][0],
                p2: triangles[t][1],
                p3: triangles[t][2],
                color: Rgba8888::WHITE,
                opa: OPA_COVER,
                grad: Gradient {
                    dir: grad_dirs[g],
                    stops: [
                        GradStop {
                            color: Rgba8888::rgb(255, 0, 255),
                            opa: OPA_COVER,
                            frac: 0,
                        },
                        GradStop {
                            color: Rgba8888::rgb(0, 255, 255),
                            opa: OPA_COVER,
                            frac: 255,
                        },
                    ],
                    stops_count: 2,
                },
            };

            draw_triangle(&mut canvas, &dsc);

            let name = format!("tri_grad_{}_{}", grad_names[g], tri_orient_names[t]);
            capture_sprite(&canvas, sprite_index, &name);
        }
    }

    // Opacity variations
    let opas = [OPA_COVER, OPA_70, OPA_40];
    let opa_names = ["100", "70", "40"];

    for o in 0..3 {
        let mut canvas = Canvas::new(SPRITE_WIDTH, SPRITE_HEIGHT);
        canvas.clear(Rgba8888::TRANSPARENT);

        let dsc = TriangleDsc {
            p1: triangles[0][0],
            p2: triangles[0][1],
            p3: triangles[0][2],
            color: Rgba8888::rgb(255, 128, 0),
            opa: opas[o],
            grad: Gradient::none(),
        };

        draw_triangle(&mut canvas, &dsc);

        let name = format!("tri_opa{}", opa_names[o]);
        capture_sprite(&canvas, sprite_index, &name);
    }
}

fn generate_lines(sprite_index: &mut usize) {
    let widths = [1, 3, 6, 10];

    // Horizontal
    for w in 0..4 {
        let mut canvas = Canvas::new(SPRITE_WIDTH, SPRITE_HEIGHT);
        canvas.clear(Rgba8888::TRANSPARENT);

        let dsc = LineDsc {
            p1: Point::new(15, 62),
            p2: Point::new(87, 62),
            width: widths[w],
            color: Rgba8888::rgb(255, 255, 255),
            opa: OPA_COVER,
            dash_width: 0,
            dash_gap: 0,
            round_start: false,
            round_end: false,
        };

        draw_line(&mut canvas, &dsc);

        let name = format!("line_hor_w{}", widths[w]);
        capture_sprite(&canvas, sprite_index, &name);
    }

    // Vertical
    for w in 0..4 {
        let mut canvas = Canvas::new(SPRITE_WIDTH, SPRITE_HEIGHT);
        canvas.clear(Rgba8888::TRANSPARENT);

        let dsc = LineDsc {
            p1: Point::new(51, 25),
            p2: Point::new(51, 100),
            width: widths[w],
            color: Rgba8888::rgb(255, 255, 0),
            opa: OPA_COVER,
            dash_width: 0,
            dash_gap: 0,
            round_start: false,
            round_end: false,
        };

        draw_line(&mut canvas, &dsc);

        let name = format!("line_ver_w{}", widths[w]);
        capture_sprite(&canvas, sprite_index, &name);
    }

    // Diagonal
    for w in 0..4 {
        let mut canvas = Canvas::new(SPRITE_WIDTH, SPRITE_HEIGHT);
        canvas.clear(Rgba8888::TRANSPARENT);

        let dsc = LineDsc {
            p1: Point::new(20, 30),
            p2: Point::new(82, 95),
            width: widths[w],
            color: Rgba8888::rgb(0, 255, 255),
            opa: OPA_COVER,
            dash_width: 0,
            dash_gap: 0,
            round_start: false,
            round_end: false,
        };

        draw_line(&mut canvas, &dsc);

        let name = format!("line_diag_w{}", widths[w]);
        capture_sprite(&canvas, sprite_index, &name);
    }

    // Dashed patterns
    let dash_configs = [[5, 3], [10, 5], [2, 2], [8, 2]];

    for d in 0..4 {
        let mut canvas = Canvas::new(SPRITE_WIDTH, SPRITE_HEIGHT);
        canvas.clear(Rgba8888::TRANSPARENT);

        let dsc = LineDsc {
            p1: Point::new(15, 62),
            p2: Point::new(87, 62),
            width: 3,
            color: Rgba8888::rgb(255, 100, 255),
            opa: OPA_COVER,
            dash_width: dash_configs[d][0],
            dash_gap: dash_configs[d][1],
            round_start: false,
            round_end: false,
        };

        draw_line(&mut canvas, &dsc);

        let name = format!("line_dash_w{}_g{}", dash_configs[d][0], dash_configs[d][1]);
        capture_sprite(&canvas, sprite_index, &name);
    }

    // Round caps
    let cap_configs = [[false, false], [true, false], [false, true], [true, true]];
    let cap_names = ["none", "start", "end", "both"];

    for c in 0..4 {
        let mut canvas = Canvas::new(SPRITE_WIDTH, SPRITE_HEIGHT);
        canvas.clear(Rgba8888::TRANSPARENT);

        let dsc = LineDsc {
            p1: Point::new(20, 40),
            p2: Point::new(82, 85),
            width: 8,
            color: Rgba8888::rgb(100, 255, 100),
            opa: OPA_COVER,
            dash_width: 0,
            dash_gap: 0,
            round_start: cap_configs[c][0],
            round_end: cap_configs[c][1],
        };

        draw_line(&mut canvas, &dsc);

        let name = format!("line_cap_{}", cap_names[c]);
        capture_sprite(&canvas, sprite_index, &name);
    }

    // Opacity
    let opas = [OPA_COVER, OPA_70, OPA_40];
    let opa_names = ["100", "70", "40"];

    for o in 0..3 {
        let mut canvas = Canvas::new(SPRITE_WIDTH, SPRITE_HEIGHT);
        canvas.clear(Rgba8888::TRANSPARENT);

        let dsc = LineDsc {
            p1: Point::new(15, 62),
            p2: Point::new(87, 62),
            width: 5,
            color: Rgba8888::rgb(255, 50, 50),
            opa: opas[o],
            dash_width: 0,
            dash_gap: 0,
            round_start: false,
            round_end: false,
        };

        draw_line(&mut canvas, &dsc);

        let name = format!("line_opa{}", opa_names[o]);
        capture_sprite(&canvas, sprite_index, &name);
    }
}

fn generate_arcs(sprite_index: &mut usize) {
    let widths = [3, 8, 15];

    // Quarter arcs at different positions
    let start_angles = [0, 90, 180, 270];

    for w in 0..3 {
        for a in 0..4 {
            let mut canvas = Canvas::new(SPRITE_WIDTH, SPRITE_HEIGHT);
            canvas.clear(Rgba8888::TRANSPARENT);

            let dsc = ArcDsc {
                center: Point::new(51, 62),
                radius: 35,
                start_angle: start_angles[a],
                end_angle: start_angles[a] + 90,
                width: widths[w],
                color: Rgba8888::rgb(255, 200, 0),
                opa: OPA_COVER,
                rounded: false,
            };

            draw_arc(&mut canvas, &dsc);

            let name = format!("arc_quarter_w{}_a{}", widths[w], start_angles[a]);
            capture_sprite(&canvas, sprite_index, &name);
        }
    }

    // Different arc spans
    let arc_spans = [45, 90, 180, 270];

    for s in 0..4 {
        let mut canvas = Canvas::new(SPRITE_WIDTH, SPRITE_HEIGHT);
        canvas.clear(Rgba8888::TRANSPARENT);

        let dsc = ArcDsc {
            center: Point::new(51, 62),
            radius: 35,
            start_angle: 0,
            end_angle: arc_spans[s],
            width: 8,
            color: Rgba8888::rgb(100, 255, 255),
            opa: OPA_COVER,
            rounded: false,
        };

        draw_arc(&mut canvas, &dsc);

        let name = format!("arc_span{}", arc_spans[s]);
        capture_sprite(&canvas, sprite_index, &name);
    }

    // Rounded ends
    for w in 0..3 {
        let mut canvas = Canvas::new(SPRITE_WIDTH, SPRITE_HEIGHT);
        canvas.clear(Rgba8888::TRANSPARENT);

        let dsc = ArcDsc {
            center: Point::new(51, 62),
            radius: 35,
            start_angle: 45,
            end_angle: 225,
            width: widths[w],
            color: Rgba8888::rgb(255, 100, 255),
            opa: OPA_COVER,
            rounded: true,
        };

        draw_arc(&mut canvas, &dsc);

        let name = format!("arc_rounded_w{}", widths[w]);
        capture_sprite(&canvas, sprite_index, &name);
    }

    // Opacity variations
    let opas = [OPA_COVER, OPA_70, OPA_40];
    let opa_names = ["100", "70", "40"];

    for o in 0..3 {
        let mut canvas = Canvas::new(SPRITE_WIDTH, SPRITE_HEIGHT);
        canvas.clear(Rgba8888::TRANSPARENT);

        let dsc = ArcDsc {
            center: Point::new(51, 62),
            radius: 35,
            start_angle: 0,
            end_angle: 270,
            width: 10,
            color: Rgba8888::rgb(255, 50, 50),
            opa: opas[o],
            rounded: false,
        };

        draw_arc(&mut canvas, &dsc);

        let name = format!("arc_opa{}", opa_names[o]);
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

    for c in 0..4 {
        let mut canvas = Canvas::new(SPRITE_WIDTH, SPRITE_HEIGHT);
        canvas.clear(Rgba8888::TRANSPARENT);

        let dsc = ArcDsc {
            center: Point::new(51, 62),
            radius: 35,
            start_angle: 0,
            end_angle: 180,
            width: 6,
            color: colors[c],
            opa: OPA_COVER,
            rounded: false,
        };

        draw_arc(&mut canvas, &dsc);

        let name = format!("arc_color_{}", color_names[c]);
        capture_sprite(&canvas, sprite_index, &name);
    }
}

fn generate_labels(sprite_index: &mut usize) {
    // Labels are stubs for now (need font rendering)
    // Generate placeholder sprites matching main.c exactly
    
    // Basic text with default font - 6 sprites
    let texts = ["A", "AB", "ABC", "Text", "123", "!@#"];
    let text_names = ["A", "AB", "ABC", "Text", "123", "sym"];

    for t in 0..6 {
        let mut canvas = Canvas::new(SPRITE_WIDTH, SPRITE_HEIGHT);
        canvas.clear(Rgba8888::TRANSPARENT);

        // Placeholder: draw white rectangle (255, 255, 255)
        let area = Area::new(30, 50, 72, 75);
        canvas.fill_area(&area, Rgba8888::rgb(255, 255, 255), OPA_COVER);

        let name = format!("label_text_{}", text_names[t]);
        capture_sprite(&canvas, sprite_index, &name);
    }

    // Text decorations - 3 sprites
    let decor_names = ["none", "underline", "strike"];
    
    for d in 0..3 {
        let mut canvas = Canvas::new(SPRITE_WIDTH, SPRITE_HEIGHT);
        canvas.clear(Rgba8888::TRANSPARENT);
        
        // Placeholder: draw yellow rectangle (255, 255, 0)
        let area = Area::new(25, 50, 77, 75);
        canvas.fill_area(&area, Rgba8888::rgb(255, 255, 0), OPA_COVER);
        
        let name = format!("label_decor_{}", decor_names[d]);
        capture_sprite(&canvas, sprite_index, &name);
    }
    
    // Letter spacing - 3 sprites
    let spacings = [0, 5, 10];
    
    for s in 0..3 {
        let mut canvas = Canvas::new(SPRITE_WIDTH, SPRITE_HEIGHT);
        canvas.clear(Rgba8888::TRANSPARENT);
        
        // Placeholder: draw cyan rectangle (100, 255, 255)
        let area = Area::new(15, 50, 87, 75);
        canvas.fill_area(&area, Rgba8888::rgb(100, 255, 255), OPA_COVER);
        
        let name = format!("label_spacing{}", spacings[s]);
        capture_sprite(&canvas, sprite_index, &name);
    }
    
    // Opacity - 3 sprites
    let opas = [OPA_COVER, OPA_70, OPA_40];
    let opa_names = ["100", "70", "40"];
    
    for o in 0..3 {
        let mut canvas = Canvas::new(SPRITE_WIDTH, SPRITE_HEIGHT);
        canvas.clear(Rgba8888::TRANSPARENT);
        
        // Placeholder: draw magenta rectangle (255, 100, 255) with opacity
        let area = Area::new(25, 50, 77, 75);
        canvas.fill_area(&area, Rgba8888::rgb(255, 100, 255), opas[o]);
        
        let name = format!("label_opa{}", opa_names[o]);
        capture_sprite(&canvas, sprite_index, &name);
    }
    
    // Different colors - 4 sprites
    let colors = [
        Rgba8888::rgb(255, 0, 0),
        Rgba8888::rgb(0, 255, 0),
        Rgba8888::rgb(0, 0, 255),
        Rgba8888::rgb(255, 128, 0),
    ];
    let color_names = ["red", "green", "blue", "orange"];
    
    for c in 0..4 {
        let mut canvas = Canvas::new(SPRITE_WIDTH, SPRITE_HEIGHT);
        canvas.clear(Rgba8888::TRANSPARENT);
        
        // Placeholder: draw colored rectangles
        let area = Area::new(30, 50, 72, 75);
        canvas.fill_area(&area, colors[c], OPA_COVER);
        
        let name = format!("label_color_{}", color_names[c]);
        capture_sprite(&canvas, sprite_index, &name);
    }
}

fn generate_blurs(sprite_index: &mut usize) {
    // Blurs are stubs for now (need convolution)
    // Generate placeholder sprites
    let blur_radii = [2, 5, 10, 15];
    let corner_radii = [0, 10, 20];
    let corner_names = ["square", "r10", "r20"];

    for b in 0..4 {
        for c in 0..3 {
            let mut canvas = Canvas::new(SPRITE_WIDTH, SPRITE_HEIGHT);
            canvas.clear(Rgba8888::TRANSPARENT);

            // Draw a gradient rect (blur would be applied to this)
            let mut dsc = RectDsc::new();
            dsc.radius = corner_radii[c];
            dsc.bg_opa = OPA_COVER;
            dsc.bg_color = Rgba8888::rgb(255, 0, 0);
            dsc.bg_grad = Gradient::horizontal(
                Rgba8888::rgb(255, 0, 0),
                Rgba8888::rgb(0, 0, 255),
            );

            let area = Area::new(20, 30, 82, 95);
            draw_rect(&mut canvas, &dsc, &area);

            // Blur would be applied here

            let name = format!("blur_r{}_{}", blur_radii[b], corner_names[c]);
            capture_sprite(&canvas, sprite_index, &name);
        }
    }
}
