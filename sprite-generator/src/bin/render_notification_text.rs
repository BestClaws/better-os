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

    let text = "Notification";
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

   
    fs::create_dir_all("sprites").expect("failed to create sprites directory");
    bmp::save_bmp(&canvas, "sprites/notification.bmp").expect("failed to write BMP");
}
