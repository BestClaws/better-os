use crate::color::Rgba8888;
use crate::math::sqrt32;
use crate::types::Area;
use crate::Rasterizer;

const BLUR_INTENSITY_BITS: u32 = 12;
const BLUR_INTENSITY_MAX: u32 = 1 << BLUR_INTENSITY_BITS;

/// Blur descriptor mirroring LVGL behaviour.
#[derive(Clone, Debug)]
pub struct BlurDsc {
    pub blur_radius: i32,
    pub corner_radius: i32,
}

impl BlurDsc {
    pub fn new(blur_radius: i32) -> Self {
        Self {
            blur_radius,
            corner_radius: 0,
        }
    }
}

/// Apply blur effect following LVGL's software renderer.
pub fn draw_blur<R>(rast: &mut R, dsc: &BlurDsc, area: &Area)
where
    R: Rasterizer,
{
    if dsc.blur_radius <= 0 {
        return;
    }

    let width = rast.width();
    let height = rast.height();
    if width == 0 || height == 0 {
        return;
    }

    let bounds = Area::new(0, 0, width as i32 - 1, height as i32 - 1);
    let Some(mut clipped) = area.intersect(&bounds) else {
        return;
    };

    let mut skip_cnt = 1;
    let clipped_size = clipped.width() * clipped.height();
    if dsc.blur_radius >= 32 && dsc.corner_radius == 0 && clipped_size > 160 * 160 {
        skip_cnt = 3;
    } else if dsc.blur_radius >= 8 {
        skip_cnt = 2;
    }

    clipped.x1 = align_up(clipped.x1, skip_cnt);
    clipped.y1 = align_up(clipped.y1, skip_cnt);
    clipped.x2 = align_down_with_margin(clipped.x2, skip_cnt);
    clipped.y2 = align_down_with_margin(clipped.y2, skip_cnt);

    if clipped.x1 > clipped.x2 || clipped.y1 > clipped.y2 {
        return;
    }

    let mut effective_radius = dsc.blur_radius / skip_cnt;
    if effective_radius <= 0 {
        effective_radius = 1;
    }

    let sample_len = (effective_radius / 2).max(1) as usize;
    let short_side = area.width().min(area.height());
    let mut radius = dsc.corner_radius;
    if radius > short_side / 2 {
        radius = short_side / 2;
    }

    let total_pixels = width * height;
    let expected_len = total_pixels * core::mem::size_of::<Rgba8888>();
    let buf_bytes = rast.buffer_mut();
    if buf_bytes.len() < expected_len {
        return;
    }

    let buffer = unsafe {
        core::slice::from_raw_parts_mut(buf_bytes.as_mut_ptr() as *mut Rgba8888, total_pixels)
    };

    let max_x_full = area.x2.min(bounds.x2);
    let max_y_full = area.y2.min(bounds.y2);

    let intensity = ((BLUR_INTENSITY_MAX * effective_radius as u32)
        / (effective_radius as u32 + 4))
        .min(BLUR_INTENSITY_MAX);

    let skip = skip_cnt;
    let skip_usize = skip as usize;

    // Column pass (down then up) to accumulate blur vertically.
    for x in (clipped.x1..=clipped.x2).step_by(skip_usize) {
        let cir_y = if radius > 0 {
            get_rounded_edge_point(area.x1, area.x2, x, radius)
        } else {
            0
        };

        let mut y_start = clipped.y1.max(area.y1 + cir_y).min(clipped.y2);
        let mut y_end = clipped.y1.max(area.y2 - cir_y).min(clipped.y2);

        y_start = align_down(y_start, skip);
        y_end = align_down(y_end, skip);

        if y_start > y_end {
            continue;
        }

        let span = ((y_end - y_start) / skip) + 1;
        let samples = sample_len.min(span as usize).max(1);

        let mut sum = init_sum(buffer, width, x, y_start, samples, 0, skip);
        let mut y_cur = y_start;
        loop {
            blur_pixel(buffer, width, x, y_cur, &mut sum, intensity);
            if y_cur == y_end {
                break;
            }
            y_cur += skip;
        }

        let mut sum_rev = init_sum(buffer, width, x, y_end, samples, 0, -skip);
        let mut y_cur = y_end;
        loop {
            blur_pixel(buffer, width, x, y_cur, &mut sum_rev, intensity);
            if y_cur == y_start {
                break;
            }
            y_cur -= skip;
        }
    }

    // Row pass (right then left) to finish the blur and fill skipped pixels.
    for y in (clipped.y1..=clipped.y2).step_by(skip_usize) {
        let cir_x = if radius > 0 {
            get_rounded_edge_point(area.y1, area.y2, y, radius)
        } else {
            0
        };

        let mut x_start = clipped.x1.max(area.x1 + cir_x).min(clipped.x2);
        let mut x_end = clipped.x1.max(area.x2 - cir_x).min(clipped.x2);

        x_start = align_down(x_start, skip);
        x_end = align_down(x_end, skip);

        if x_start > x_end {
            continue;
        }

        let span = ((x_end - x_start) / skip) + 1;
        let samples = sample_len.min(span as usize).max(1);

        if span > 1 {
            let mut sum = init_sum(buffer, width, x_start, y, samples, skip, 0);
            let mut x_cur = x_start;
            for _ in 0..(span - 1) {
                blur_pixel(buffer, width, x_cur, y, &mut sum, intensity);
                x_cur += skip;
            }
        }

        let mut sum_rev = init_sum(buffer, width, x_end, y, samples, -skip, 0);
        let mut cur_x_index = x_start;
        let mut x_cur = x_end;
        loop {
            blur_pixel(buffer, width, x_cur, y, &mut sum_rev, intensity);

            if skip > 1 {
                let right_edge = (x_cur + skip - 1).min(max_x_full);
                for target_x in (x_cur + 1)..=right_edge {
                    copy_pixel(buffer, width, x_cur, target_x, y);
                }
            }

            if skip > 1 && cur_x_index + skip > x_end {
                let right_edge = (x_end + skip - 1).min(max_x_full);
                for extra in 1..skip {
                    let target_y = y + extra;
                    if target_y > max_y_full {
                        break;
                    }
                    copy_row(buffer, width, y, target_y, x_start, right_edge);
                }
            }

            if cur_x_index == x_end {
                break;
            }
            x_cur -= skip;
            cur_x_index += skip;
        }
    }

}

fn init_sum(
    buffer: &[Rgba8888],
    width: usize,
    mut x: i32,
    mut y: i32,
    samples: usize,
    step_x: i32,
    step_y: i32,
) -> [u32; 3] {
    let mut sum = [0u32; 3];
    for _ in 0..samples {
        let idx = pixel_index(width, x, y);
        let color = buffer[idx];
        sum[0] += color.r() as u32;
        sum[1] += color.g() as u32;
        sum[2] += color.b() as u32;
        x += step_x;
        y += step_y;
    }
    for channel in sum.iter_mut() {
        *channel = (*channel << BLUR_INTENSITY_BITS) / samples as u32;
    }
    sum
}

fn blur_pixel(
    buffer: &mut [Rgba8888],
    width: usize,
    x: i32,
    y: i32,
    sum: &mut [u32; 3],
    intensity: u32,
) {
    let idx = pixel_index(width, x, y);
    let color = buffer[idx];
    let r = blur_channel(&mut sum[0], color.r(), intensity);
    let g = blur_channel(&mut sum[1], color.g(), intensity);
    let b = blur_channel(&mut sum[2], color.b(), intensity);
    buffer[idx] = Rgba8888::rgba(r, g, b, color.a());
}

fn blur_channel(sum: &mut u32, value: u8, intensity: u32) -> u8 {
    let inv = BLUR_INTENSITY_MAX - intensity;
    *sum = ((*sum * intensity) >> BLUR_INTENSITY_BITS) + (value as u32 * inv);
    (*sum >> BLUR_INTENSITY_BITS) as u8
}

fn copy_pixel(buffer: &mut [Rgba8888], width: usize, src_x: i32, dst_x: i32, y: i32) {
    let src_idx = pixel_index(width, src_x, y);
    let dst_idx = pixel_index(width, dst_x, y);
    let src = buffer[src_idx];
    let dst_alpha = buffer[dst_idx].a();
    buffer[dst_idx] = Rgba8888::rgba(src.r(), src.g(), src.b(), dst_alpha);
}

fn copy_row(
    buffer: &mut [Rgba8888],
    width: usize,
    src_y: i32,
    dst_y: i32,
    x_start: i32,
    x_end: i32,
) {
    for x in x_start..=x_end {
        let src_idx = pixel_index(width, x, src_y);
        let dst_idx = pixel_index(width, x, dst_y);
        buffer[dst_idx] = buffer[src_idx];
    }
}

fn pixel_index(width: usize, x: i32, y: i32) -> usize {
    (y as usize) * width + x as usize
}

fn align_up(value: i32, align: i32) -> i32 {
    if align <= 1 {
        return value;
    }
    let remainder = value % align;
    if remainder == 0 {
        value
    } else {
        value + (align - remainder)
    }
}

fn align_down(value: i32, align: i32) -> i32 {
    if align <= 1 {
        return value;
    }
    value - (value % align)
}

fn align_down_with_margin(value: i32, align: i32) -> i32 {
    if align <= 1 {
        return value;
    }
    ((value - (align - 1)) / align) * align
}

fn get_rounded_edge_point(p_start: i32, p_end: i32, p: i32, r: i32) -> i32 {
    if r <= 0 {
        return 0;
    }

    let mut p_local = p;
    if p_local < p_start + r {
        p_local = r - (p_local - p_start);
    } else if p_local > p_end - r {
        p_local = r - (p_end - p_local);
    } else {
        return 0;
    }

    let inside = (r as i64 * r as i64) - (p_local as i64 * p_local as i64);
    if inside <= 0 {
        return r;
    }
    let res = sqrt32(inside as u32) as i32;
    r - res
}
