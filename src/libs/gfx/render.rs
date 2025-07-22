use core::iter::Iterator;
use embedded_graphics::{
    prelude::{DrawTarget, Point, Size},
    Drawable, Pixel,
};
use embedded_graphics::prelude::{Dimensions, OriginDimensions};
use embedded_graphics::primitives::Triangle;
use micromath::F32Ext;

use crate::system::ui::canvas::Rgb332;
use super::math::{Quaternion, Vec3};
use super::model::{Model, MAX_TRIANGLES, MAX_VERTICES};

pub struct RenderOptions {
    pub fov_deg: f32,
    pub light_dir: Vec3,
    pub intensity_range: (f32, f32),
}

fn project(v: Vec3, fov_deg: f32, width: u32, height: u32) -> Option<Point> {
    if v.2 <= 0.5 {
        return None;
    }

    let fov_rad = fov_deg.to_radians();
    let f = 1.0 / (fov_rad / 2.0).tan();

    let x_proj = (v.0 * f) / v.2;
    let y_proj = (v.1 * f) / v.2;

    let aspect = width as f32 / height as f32;
    let x = ((x_proj / aspect + 1.0) * (width as f32 / 2.0)) as i32;
    let y = ((1.0 - y_proj) * (height as f32 / 2.0)) as i32;

    if x < 0 || x >= width as i32 || y < 0 || y >= height as i32 {
        return None;
    }

    Some(Point::new(x, y))
}

fn get_grayscale_color(intensity: f32) -> Rgb332 {
    let palette = [
        Rgb332::new(0, 0, 0),     // black
        Rgb332::new(2, 2, 1),     // dark gray
        Rgb332::new(4, 4, 2),     // mid gray
        Rgb332::new(6, 6, 3),     // light gray
        Rgb332::new(7, 7, 3),     // white
    ];

    let clamped = intensity.clamp(0.0, 1.0);
    let index = (clamped * (palette.len() - 1) as f32).round() as usize;
    palette[index.min(palette.len() - 1)]
}

struct ShadedTriangle {
    p0: Point,
    p1: Point,
    p2: Point,
    color: Rgb332,
}

impl OriginDimensions for ShadedTriangle {
    fn size(&self) -> Size {
        Triangle::new(self.p0, self.p1, self.p2).bounding_box().size
    }
}

impl Drawable for ShadedTriangle {
    type Color = Rgb332;
    type Output = ();

    fn draw<D: DrawTarget<Color = Rgb332>>(&self, target: &mut D) -> Result<Self::Output, D::Error> {
        let mut points = [self.p0, self.p1, self.p2];
        points.sort_by_key(|p| p.y);
        let (top, mid, bot) = (points[0], points[1], points[2]);

        if top.y == bot.y || (top.x == mid.x && mid.x == bot.x) {
            return Ok(());
        }

        let canvas_width = target.bounding_box().size.width as i32;
        let canvas_height = target.bounding_box().size.height as i32;

        let x0 = top.x as f32;
        let x1 = mid.x as f32;
        let x2 = bot.x as f32;
        let y0 = top.y as f32;
        let y1 = mid.y as f32;
        let y2 = bot.y as f32;

        let slope_top_bot = if y2 != y0 { (x2 - x0) / (y2 - y0) } else { 0.0 };
        let slope_top_mid = if y1 != y0 { (x1 - x0) / (y1 - y0) } else { 0.0 };
        let slope_mid_bot = if y2 != y1 { (x2 - x1) / (y2 - y1) } else { 0.0 };

        let pixels_top_half = (top.y..mid.y).flat_map(move |y| {
            let x_start = (x0 + slope_top_bot * (y as f32 - y0)).round() as i32;
            let x_end = (x0 + slope_top_mid * (y as f32 - y0)).round() as i32;
            let (x_min, x_max) = if x_start <= x_end { (x_start, x_end + 1) } else { (x_end, x_start + 1) };
            let x_min = x_min.max(0).min(canvas_width);
            let x_max = x_max.max(0).min(canvas_width);
            (x_min..x_max).map(move |x| Pixel(Point::new(x, y), self.color))
        });

        let pixels_bottom_half = (mid.y..=bot.y).flat_map(move |y| {
            let x_start = (x0 + slope_top_bot * (y as f32 - y0)).round() as i32;
            let x_end = (x1 + slope_mid_bot * (y as f32 - y1)).round() as i32;
            let (x_min, x_max) = if x_start <= x_end { (x_start, x_end + 1) } else { (x_end, x_start + 1) };
            let x_min = x_min.max(0).min(canvas_width);
            let x_max = x_max.max(0).min(canvas_width);
            (x_min..x_max).map(move |x| Pixel(Point::new(x, y), self.color))
        });

        target.draw_iter(pixels_top_half.chain(pixels_bottom_half))?;
        Ok(())
    }
}

pub fn draw_model<D: DrawTarget<Color = Rgb332>>(
    display: &mut D,
    model: &Model,
    origin: Vec3,
    rotation: Quaternion,
    width: u32,
    height: u32,
    options: &RenderOptions,
) -> Result<(), D::Error> {
    let light_dir = options.light_dir.normalize();

    let mut projected = [None; MAX_VERTICES];
    let mut world_vertices = [Vec3(0.0, 0.0, 0.0); MAX_VERTICES];

    for i in 0..model.vertex_count {
        let rotated = rotation.rotate_vector(model.vertices[i]);
        let world = Vec3(origin.0 + rotated.0, origin.1 + rotated.1, origin.2 + rotated.2);
        world_vertices[i] = world;
        projected[i] = project(world, options.fov_deg, width, height);
    }

    let mut triangles = [(0, 0.0, Vec3(0.0, 0.0, 0.0)); MAX_TRIANGLES];
    for i in 0..model.triangle_count {
        let t = model.triangles[i];
        let z_avg = (world_vertices[t.vertices[0]].2 +
            world_vertices[t.vertices[1]].2 +
            world_vertices[t.vertices[2]].2) / 3.0;
        triangles[i] = (i, z_avg, t.normal);
    }

    triangles[0..model.triangle_count].sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(core::cmp::Ordering::Equal));

    for (tri_idx, _z_avg, normal) in triangles.iter().take(model.triangle_count) {
        let t = model.triangles[*tri_idx];
        if let (Some(p0), Some(p1), Some(p2)) = (
            projected[t.vertices[0]],
            projected[t.vertices[1]],
            projected[t.vertices[2]],
        ) {
            let rotated_normal = rotation.rotate_vector(*normal);
            let raw_intensity = rotated_normal.dot(light_dir).max(0.0).clamp(0.0, 1.0);
            let intensity = options.intensity_range.0 + (options.intensity_range.1 - options.intensity_range.0) * raw_intensity;

            let color = get_grayscale_color(intensity);
            ShadedTriangle { p0, p1, p2, color }.draw(display)?;
        }
    }

    Ok(())
}
