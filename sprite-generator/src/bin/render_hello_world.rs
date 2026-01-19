#[path = "../canvas.rs"]
mod canvas;
#[path = "../bmp.rs"]
mod bmp;

use canvas::Canvas;
use rust_gfx::color::Rgba8888;
use rust_gfx::primitives::label::{draw_label, measure_text_with_font, LabelDsc, FontId};
use rust_gfx::types::Area;
use rust_gfx::types::OPA_COVER;
use std::fs;

fn main() {
    let mut canvas = Canvas::new(160, 60);
    canvas.clear(Rgba8888::rgb(0, 0, 0));

    let text = "hello world";
    let font = FontId::Montserrat16;

    let mut label = LabelDsc::new(text.to_string());
    label.font = font;
    label.color = Rgba8888::rgb(255, 255, 255);
    label.opa = OPA_COVER;

    let text_width = measure_text_with_font(text, label.letter_space, font);
    let text_height = rust_gfx::primitives::label::line_height_for_font(font);
    let origin_x = ((canvas.width as i32 - text_width) / 2).max(0);
    let origin_y = ((canvas.height as i32 - text_height) / 2).max(0);
    let area = Area::new(
        origin_x,
        origin_y,
        origin_x + text_width - 1,
        origin_y + text_height - 1,
    );

    draw_label(&mut canvas, &label, &area);

    let tight = measure_text_with_font("helloworld", label.letter_space, font);
    println!(
        "measured widths -> with space: {text_width}, without space: {tight}, delta: {}",
        text_width - tight
    );

    let mut has_pixel = vec![false; canvas.width];
    for y in 0..canvas.height {
        for x in 0..canvas.width {
            if canvas.get_pixel(x as i32, y as i32).a() > 0 {
                has_pixel[x] = true;
            }
        }
    }

    let mut max_gap = 0;
    let mut current_gap = 0;
    let mut seen_pixel = false;
    for &column in has_pixel.iter() {
        if column {
            if seen_pixel && current_gap > max_gap {
                max_gap = current_gap;
            }
            seen_pixel = true;
            current_gap = 0;
        } else if seen_pixel {
            current_gap += 1;
        }
    }
    if seen_pixel && current_gap > max_gap {
        max_gap = current_gap;
    }

    println!("max gap columns between drawn pixels: {max_gap}");
    let occupancy: String = has_pixel
        .iter()
        .map(|&p| if p { '#' } else { '.' })
        .collect();
    println!("column occupancy: {occupancy}");
    if let (Some(first), Some(last)) = (
        has_pixel.iter().position(|&p| p),
        has_pixel.iter().rposition(|&p| p),
    ) {
        let empty: Vec<usize> = has_pixel[first..=last]
            .iter()
            .enumerate()
            .filter_map(|(idx, &p)| if !p { Some(first + idx) } else { None })
            .collect();
        println!("empty column indices between first/last pixels: {empty:?}");
    }

    fs::create_dir_all("sprites").expect("failed to create sprites directory");
    bmp::save_bmp(&canvas, "sprites/hello_world.bmp").expect("failed to write BMP");
}
