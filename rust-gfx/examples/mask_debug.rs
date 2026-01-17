use rust_gfx::masks::{apply_masks, LineMask, LineSide, MaskRef};
use rust_gfx::primitives::line::LineDsc;
use rust_gfx::primitives::rectangle::RectDsc;
use rust_gfx::primitives::triangle::TriangleDsc;
use rust_gfx::primitives::*;
use rust_gfx::{Area, Canvas, Point, Rgba8888, RADIUS_CIRCLE};

fn main() {
    let mut canvas = Canvas::new(102, 125);
    canvas.clear(Rgba8888::TRANSPARENT);

    let mut tri_fill = TriangleDsc::new(Point::new(51, 30), Point::new(25, 90), Point::new(77, 90));
    tri_fill.color = Rgba8888::rgb(255, 100, 100);
    draw_triangle(&mut canvas, &tri_fill);

    let mut line_dsc = LineDsc::new(Point::new(0, 0), Point::new(0, 0));
    line_dsc.width = 2;
    line_dsc.color = Rgba8888::WHITE;
    line_dsc.round_start = true;
    line_dsc.round_end = true;

    let tri_points = [Point::new(51, 30), Point::new(25, 90), Point::new(77, 90)];
    let edge_colors = [Rgba8888::WHITE; 3];
    for edge in 0..3 {
        line_dsc.p1 = tri_points[edge];
        line_dsc.p2 = tri_points[(edge + 1) % 3];
        line_dsc.color = edge_colors[edge];
        draw_line(&mut canvas, &line_dsc);
        let p = canvas.get_pixel(24, 89).to_u32();
        let q = canvas.get_pixel(24, 90).to_u32();
        println!("after edge {edge}: (24,89)=0x{p:08x} (24,90)=0x{q:08x}");
    }

    let p1 = tri_points[0];
    let p2 = tri_points[1];
    let dx = p2.x - p1.x;
    let dy = p2.y - p1.y;
    let flat = dx.abs() > dy.abs();
    let w = line_dsc.width;
    let w_half0 = w >> 1;
    let w_half1 = w_half0 + (w & 1);

    let (mask_left, mask_right) = if flat {
        if dx > 0 {
            (
                LineMask::from_points(
                    Point::new(p1.x, p1.y - w_half0),
                    Point::new(p2.x, p2.y - w_half0),
                    LineSide::Left,
                ),
                LineMask::from_points(
                    Point::new(p1.x, p1.y + w_half1),
                    Point::new(p2.x, p2.y + w_half1),
                    LineSide::Right,
                ),
            )
        } else {
            (
                LineMask::from_points(
                    Point::new(p1.x, p1.y + w_half1),
                    Point::new(p2.x, p2.y + w_half1),
                    LineSide::Left,
                ),
                LineMask::from_points(
                    Point::new(p1.x, p1.y - w_half0),
                    Point::new(p2.x, p2.y - w_half0),
                    LineSide::Right,
                ),
            )
        }
    } else {
        (
            LineMask::from_points(
                Point::new(p1.x + w_half1, p1.y),
                Point::new(p2.x + w_half1, p2.y),
                LineSide::Left,
            ),
            LineMask::from_points(
                Point::new(p1.x - w_half0, p1.y),
                Point::new(p2.x - w_half0, p2.y),
                LineSide::Right,
            ),
        )
    };

    let mask_top = LineMask::from_points(p1, Point::new(p1.x - dy, p1.y + dx), LineSide::Bottom);
    let mask_bottom = LineMask::from_points(p2, Point::new(p2.x - dy, p2.y + dx), LineSide::Top);

    let blend_area = Area::new(
        p1.x.min(p2.x) - w,
        p1.y.min(p2.y) - w,
        p1.x.max(p2.x) + w,
        p1.y.max(p2.y) + w,
    );

    let draw_width = (blend_area.x2 - blend_area.x1 + 1) as usize;
    let mut mask_buf = vec![255u8; draw_width];
    let masks = [
        MaskRef::Line(&mask_left),
        MaskRef::Line(&mask_right),
        MaskRef::Line(&mask_top),
        MaskRef::Line(&mask_bottom),
    ];

    let target_y = 89;
    let target_x = 24;
    let _ = apply_masks(&masks, &mut mask_buf, blend_area.x1, target_y);
    let idx = (target_x - blend_area.x1) as usize;
    println!(
        "mask coverage at (24,89) from left edge = {} (blend_area.x1={}, w={})",
        mask_buf[idx], blend_area.x1, w
    );

    let pix_2489 = canvas.get_pixel(24, 89).to_u32();
    let pix_2490 = canvas.get_pixel(24, 90).to_u32();
    println!("after full draw: (24,89)=0x{pix_2489:08x} (24,90)=0x{pix_2490:08x}");

    let mut canvas_cap = Canvas::new(4, 4);
    canvas_cap.clear(Rgba8888::TRANSPARENT);

    let mut cap_dsc = RectDsc::new();
    cap_dsc.bg_color = Rgba8888::WHITE;
    cap_dsc.bg_opa = 255;
    cap_dsc.radius = RADIUS_CIRCLE;

    let cap_area = Area::new(0, 0, 1, 1);
    draw_rect(&mut canvas_cap, &cap_dsc, &cap_area);

    let cap00 = canvas_cap.get_pixel(0, 0).to_u32();
    let cap01 = canvas_cap.get_pixel(0, 1).to_u32();
    println!("cap only: (0,0)=0x{cap00:08x} (0,1)=0x{cap01:08x}");
}
