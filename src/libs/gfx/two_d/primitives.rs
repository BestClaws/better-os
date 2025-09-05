use crate::libs::gfx::two_d::raster::Rasterizer;
use crate::libs::gfx::two_d::types::{Point, Rect, Rgb565, Rgba8888, Size};
use micromath::F32Ext;

// Re-implement by delegating to the moved algorithms from UI gfx
pub fn fill_rect(r: &mut dyn Rasterizer, rect: Rect, color: Rgb565) {
    let clip = Rect::new(Point::zero(), Size::new(r.width(), r.height()));
    if let Some(rc) = rect.intersection(&clip) {
        for y in rc.top_left.y..=rc.bottom() {
            for x in rc.top_left.x..=rc.right() {
                r.set_pixel(x, y, color);
            }
        }
    }
}

pub fn draw_line_aa(r: &mut dyn Rasterizer, p0: Point, p1: Point, color: Rgb565) {
    fn ipart(x: f32) -> i32 {
        x.floor() as i32
    }
    fn round(x: f32) -> i32 {
        (x + 0.5).floor() as i32
    }
    fn fpart(x: f32) -> f32 {
        x - x.floor()
    }
    fn rfpart(x: f32) -> f32 {
        1.0 - fpart(x)
    }
    let mut x0 = p0.x as f32;
    let mut y0 = p0.y as f32;
    let mut x1 = p1.x as f32;
    let mut y1 = p1.y as f32;
    let steep = (y1 - y0).abs() > (x1 - x0).abs();
    if steep {
        core::mem::swap(&mut x0, &mut y0);
        core::mem::swap(&mut x1, &mut y1);
    }
    if x0 > x1 {
        core::mem::swap(&mut x0, &mut x1);
        core::mem::swap(&mut y0, &mut y1);
    }
    let dx = x1 - x0;
    let dy = y1 - y0;
    let gradient = if dx == 0.0 { 1.0 } else { dy / dx };
    let xend = round(x0) as f32;
    let yend = y0 + gradient * (xend - x0);
    let xgap = rfpart(x0 + 0.5);
    let xpxl1 = xend as i32;
    let ypxl1 = ipart(yend);
    plot(r, steep, xpxl1, ypxl1, color, rfpart(yend) * xgap);
    plot(r, steep, xpxl1, ypxl1 + 1, color, fpart(yend) * xgap);
    let mut intery = yend + gradient;
    let xend2 = round(x1) as f32;
    let yend2 = y1 + gradient * (xend2 - x1);
    let xgap2 = fpart(x1 + 0.5);
    let xpxl2 = xend2 as i32;
    let ypxl2 = ipart(yend2);
    for x in (xpxl1 + 1)..xpxl2 {
        plot(r, steep, x, ipart(intery), color, rfpart(intery));
        plot(r, steep, x, ipart(intery) + 1, color, fpart(intery));
        intery += gradient;
    }
    plot(r, steep, xpxl2, ypxl2, color, rfpart(yend2) * xgap2);
    plot(r, steep, xpxl2, ypxl2 + 1, color, fpart(yend2) * xgap2);
    fn plot(r: &mut dyn Rasterizer, steep: bool, x: i32, y: i32, color: Rgb565, a: f32) {
        let a_u8 = (a.clamp(0.0, 1.0) * 255.0) as u8;
        if steep {
            r.blend_pixel(y, x, color, a_u8)
        } else {
            r.blend_pixel(x, y, color, a_u8)
        }
    }
}

pub fn draw_line_rgba_aa(r: &mut dyn Rasterizer, p0: Point, p1: Point, color: Rgba8888) {
    let base = color.to_rgb565();
    let a_base = color.a as f32 / 255.0;
    fn ipart(x: f32) -> i32 {
        x.floor() as i32
    }
    fn round(x: f32) -> i32 {
        (x + 0.5).floor() as i32
    }
    fn fpart(x: f32) -> f32 {
        x - x.floor()
    }
    fn rfpart(x: f32) -> f32 {
        1.0 - fpart(x)
    }
    let mut x0 = p0.x as f32;
    let mut y0 = p0.y as f32;
    let mut x1 = p1.x as f32;
    let mut y1 = p1.y as f32;
    let steep = (y1 - y0).abs() > (x1 - x0).abs();
    if steep {
        core::mem::swap(&mut x0, &mut y0);
        core::mem::swap(&mut x1, &mut y1);
    }
    if x0 > x1 {
        core::mem::swap(&mut x0, &mut x1);
        core::mem::swap(&mut y0, &mut y1);
    }
    let dx = x1 - x0;
    let dy = y1 - y0;
    let gradient = if dx == 0.0 { 1.0 } else { dy / dx };
    let xend = round(x0) as f32;
    let yend = y0 + gradient * (xend - x0);
    let xgap = rfpart(x0 + 0.5);
    let xpxl1 = xend as i32;
    let ypxl1 = ipart(yend);
    plot(r, steep, xpxl1, ypxl1, base, rfpart(yend) * xgap * a_base);
    plot(
        r,
        steep,
        xpxl1,
        ypxl1 + 1,
        base,
        fpart(yend) * xgap * a_base,
    );
    let mut intery = yend + gradient;
    let xend2 = round(x1) as f32;
    let yend2 = y1 + gradient * (xend2 - x1);
    let xgap2 = fpart(x1 + 0.5);
    let xpxl2 = xend2 as i32;
    let ypxl2 = ipart(yend2);
    for x in (xpxl1 + 1)..xpxl2 {
        plot(r, steep, x, ipart(intery), base, rfpart(intery) * a_base);
        plot(r, steep, x, ipart(intery) + 1, base, fpart(intery) * a_base);
        intery += gradient;
    }
    plot(r, steep, xpxl2, ypxl2, base, rfpart(yend2) * xgap2 * a_base);
    plot(
        r,
        steep,
        xpxl2,
        ypxl2 + 1,
        base,
        fpart(yend2) * xgap2 * a_base,
    );
    fn plot(r: &mut dyn Rasterizer, steep: bool, x: i32, y: i32, color: Rgb565, a: f32) {
        let a_u8 = (a.clamp(0.0, 1.0) * 255.0) as u8;
        if steep {
            r.blend_pixel(y, x, color, a_u8)
        } else {
            r.blend_pixel(x, y, color, a_u8)
        }
    }
}

pub fn draw_circle_aa(r: &mut dyn Rasterizer, center: Point, radius: i32, color: Rgb565) {
    let mut x = radius;
    let mut y = 0;
    let mut err = 1 - x;
    while x >= y {
        circle_points_aa(r, center, x, y, color);
        y += 1;
        if err < 0 {
            err += 2 * y + 1;
        } else {
            x -= 1;
            err += 2 * (y - x) + 1;
        }
    }
}
pub fn fill_circle(r: &mut dyn Rasterizer, center: Point, radius: i32, color: Rgb565) {
    let r2 = radius * radius;
    for dy in -radius..=radius {
        for dx in -radius..=radius {
            if dx * dx + dy * dy <= r2 {
                r.set_pixel(center.x + dx, center.y + dy, color);
            }
        }
    }
}
fn circle_points_aa(r: &mut dyn Rasterizer, c: Point, x: i32, y: i32, color: Rgb565) {
    let pts = [
        (c.x + x, c.y + y),
        (c.x - x, c.y + y),
        (c.x + x, c.y - y),
        (c.x - x, c.y - y),
        (c.x + y, c.y + x),
        (c.x - y, c.y + x),
        (c.x + y, c.y - x),
        (c.x - y, c.y - x),
    ];
    for &(px, py) in &pts {
        r.blend_pixel(px, py, color, 255);
    }
}

pub fn draw_rect_outline_aa(r: &mut dyn Rasterizer, rect: Rect, thickness: i32, color: Rgb565) {
    let t = thickness.max(1);
    for dy in 0..t {
        draw_line_aa(
            r,
            Point::new(rect.top_left.x, rect.top_left.y + dy),
            Point::new(rect.right(), rect.top_left.y + dy),
            color,
        );
    }
    for dy in 0..t {
        draw_line_aa(
            r,
            Point::new(rect.top_left.x, rect.bottom() - dy),
            Point::new(rect.right(), rect.bottom() - dy),
            color,
        );
    }
    for dx in 0..t {
        draw_line_aa(
            r,
            Point::new(rect.top_left.x + dx, rect.top_left.y),
            Point::new(rect.top_left.x + dx, rect.bottom()),
            color,
        );
    }
    for dx in 0..t {
        draw_line_aa(
            r,
            Point::new(rect.right() - dx, rect.top_left.y),
            Point::new(rect.right() - dx, rect.bottom()),
            color,
        );
    }
}

pub fn fill_rounded_rect(r: &mut dyn Rasterizer, rect: Rect, radius: i32, color: Rgb565) {
    let rx = radius
        .max(0)
        .min(rect.size.width as i32 / 2)
        .min(rect.size.height as i32 / 2);
    let inner = Rect::new(
        Point::new(rect.top_left.x + rx, rect.top_left.y),
        Size::new((rect.size.width as i32 - 2 * rx) as u32, rect.size.height),
    );
    fill_rect(r, inner, color);
    let side_h = (rect.size.height as i32 - 2 * rx).max(0) as u32;
    if side_h > 0 {
        fill_rect(
            r,
            Rect::new(
                Point::new(rect.top_left.x, rect.top_left.y + rx),
                Size::new(rx as u32, side_h),
            ),
            color,
        );
        fill_rect(
            r,
            Rect::new(
                Point::new(rect.right() - rx + 1, rect.top_left.y + rx),
                Size::new(rx as u32, side_h),
            ),
            color,
        );
    }
    fill_quarter_circle(
        r,
        Point::new(rect.top_left.x + rx, rect.top_left.y + rx),
        rx,
        color,
        0,
    );
    fill_quarter_circle(
        r,
        Point::new(rect.right() - rx + 1, rect.top_left.y + rx),
        rx,
        color,
        1,
    );
    fill_quarter_circle(
        r,
        Point::new(rect.top_left.x + rx, rect.bottom() - rx + 1),
        rx,
        color,
        2,
    );
    fill_quarter_circle(
        r,
        Point::new(rect.right() - rx + 1, rect.bottom() - rx + 1),
        rx,
        color,
        3,
    );
}

fn fill_quarter_circle(
    r: &mut dyn Rasterizer,
    center: Point,
    radius: i32,
    color: Rgb565,
    quadrant: u8,
) {
    let r2 = radius * radius;
    for dy in 0..=radius {
        for dx in 0..=radius {
            let dist_sq = dx * dx + dy * dy;
            if dist_sq <= r2 {
                // Calculate distance from circle edge for anti-aliasing
                let dist = (dist_sq as f32).sqrt();
                let dist_from_edge = radius as f32 - dist;
                let alpha = if dist_from_edge <= 1.0 {
                    dist_from_edge.clamp(0.0, 1.0)
                } else {
                    1.0
                };
                
                let alpha_u8 = (alpha * 255.0) as u8;
                match quadrant {
                    0 => r.blend_pixel(center.x - dx, center.y - dy, color, alpha_u8),
                    1 => r.blend_pixel(center.x + dx, center.y - dy, color, alpha_u8),
                    2 => r.blend_pixel(center.x - dx, center.y + dy, color, alpha_u8),
                    _ => r.blend_pixel(center.x + dx, center.y + dy, color, alpha_u8),
                }
            }
        }
    }
}

pub fn draw_arc_aa(
    r: &mut dyn Rasterizer,
    center: Point,
    radius: i32,
    start_angle_rad: f32,
    end_angle_rad: f32,
    color: Rgb565,
) {
    // Use more steps for smoother arcs, especially for larger radii
    let angle_diff = (end_angle_rad - start_angle_rad).abs();
    let steps = (radius as f32 * angle_diff * 2.0).max(32.0) as i32;
    
    // Draw the arc by plotting individual pixels with anti-aliasing
    for i in 0..=steps {
        let t = i as f32 / steps as f32;
        let ang = start_angle_rad + (end_angle_rad - start_angle_rad) * t;
        let x_f = center.x as f32 + radius as f32 * ang.cos();
        let y_f = center.y as f32 + radius as f32 * ang.sin();
        
        // Calculate sub-pixel position for anti-aliasing
        let x = x_f.floor() as i32;
        let y = y_f.floor() as i32;
        let fx = x_f - x as f32;
        let fy = y_f - y as f32;
        
        // Plot the main pixel
        let alpha = ((1.0 - fx) * (1.0 - fy) * 255.0) as u8;
        r.blend_pixel(x, y, color, alpha);
        
        // Plot adjacent pixels for better anti-aliasing
        if fx > 0.0 {
            let alpha = (fx * (1.0 - fy) * 255.0) as u8;
            r.blend_pixel(x + 1, y, color, alpha);
        }
        if fy > 0.0 {
            let alpha = ((1.0 - fx) * fy * 255.0) as u8;
            r.blend_pixel(x, y + 1, color, alpha);
        }
        if fx > 0.0 && fy > 0.0 {
            let alpha = (fx * fy * 255.0) as u8;
            r.blend_pixel(x + 1, y + 1, color, alpha);
        }
    }
}

pub fn draw_arc(
    r: &mut dyn Rasterizer,
    center: Point,
    radius: i32,
    start_angle_rad: f32,
    end_angle_rad: f32,
    color: Rgb565,
) {
    let steps = (radius as f32 * (end_angle_rad - start_angle_rad).abs()).max(16.0) as i32;
    for i in 0..=steps {
        let t = i as f32 / steps as f32;
        let ang = start_angle_rad + (end_angle_rad - start_angle_rad) * t;
        let x = center.x + (radius as f32 * ang.cos()) as i32;
        let y = center.y + (radius as f32 * ang.sin()) as i32;
        r.set_pixel(x, y, color);
    }
}
