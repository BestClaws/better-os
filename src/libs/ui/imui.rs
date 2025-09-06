use crate::libs::gfx::two_d::{Rasterizer, Rect, Point, Size, Rgb565};
use crate::libs::gfx::two_d::{TextRenderer, FONT_8X8};
use super::style::{Fill, Stroke, CornerRadii, Color};
use super::painter::Painter;

#[derive(Clone, Copy, Default)]
pub struct ImInput {
    pub pointer_down: bool,
    pub pointer_released: bool,
    pub pointer_pos: Option<Point>,
}

#[derive(Clone, Copy, Default)]
pub struct Response { clicked: bool }
impl Response { pub fn clicked(&self) -> bool { self.clicked } }

pub struct ImUi<'a> {
    pub raster: &'a mut dyn Rasterizer,
    input: ImInput,
    cursor: Point,
    line_height: i32,
    max_width: u32,
    theme_bg: Rgb565,
}

impl<'a> ImUi<'a> {
    pub fn new_fullscreen(raster: &'a mut dyn Rasterizer, input: ImInput) -> Self {
        let max_width = raster.width();
        Self {
            raster,
            input,
            cursor: Point::new(12, 12),
            line_height: 0,
            max_width,
            theme_bg: Rgb565::from_rgb(0, 0, 0),
        }
    }

    pub fn clear_background(&mut self, color: Rgb565) {
        use crate::libs::gfx::two_d::primitives::fill_rect;
        let rect = Rect::new(Point::new(0, 0), Size::new(self.raster.width(), self.raster.height()));
        fill_rect(self.raster, rect, color);
        self.theme_bg = color;
    }

    fn next_rect(&mut self, size: Size) -> Rect {
        let x = self.cursor.x;
        let y = self.cursor.y;
        let rect = Rect::new(Point::new(x, y), size);
        self.cursor.x += size.width as i32 + 8;
        self.line_height = self.line_height.max(size.height as i32);
        rect
    }

    fn newline(&mut self) {
        self.cursor.x = 12;
        self.cursor.y += self.line_height + 8;
        self.line_height = 0;
    }

    pub fn add_label(&mut self, text: &str) {
        // Measure approx using FONT_8X8
        let width = (text.len() as u32) * 8;
        let height = 8u32;
        let rect = self.next_rect(Size::new(width, height));
        let mut tr = TextRenderer::new(&FONT_8X8).with_color(Rgb565::from_rgb(255, 255, 255)).with_anti_alias(true);
        tr.draw_text(self.raster, rect.top_left, text);
    }

    pub fn add_label_newline(&mut self, text: &str) { self.add_label(text); self.newline(); }

    pub fn add_button(&mut self, text: &str) -> Response {
        // Button sizing with padding
        let label_w = (text.len() as u32) * 8;
        let label_h = 8u32;
        let pad = 8u32;
        let size = Size::new(label_w + pad * 2, label_h + pad * 2);
        let rect = self.next_rect(size);

        // Draw button
        let mut painter = Painter::new(self.raster);
        let bg = Fill { color: Color::GRAY_20 };
        let border = Stroke { color: Color::GRAY_40, thickness: 1 };
        let corner = CornerRadii { uniform: 6 };
        painter.rect(rect, Some(bg), Some(border), corner);

        let text_pos = Point::new(
            rect.top_left.x + pad as i32,
            rect.top_left.y + pad as i32,
        );
        let mut tr = TextRenderer::new(&FONT_8X8).with_color(Rgb565::from_rgb(255, 255, 255)).with_anti_alias(true);
        tr.draw_text(self.raster, text_pos, text);

        // Input
        let clicked = if let Some(p) = self.input.pointer_pos {
            // consider a click when we saw a release inside rect
            self.input.pointer_released &&
                p.x >= rect.top_left.x && p.y >= rect.top_left.y && p.x <= rect.right() && p.y <= rect.bottom()
        } else { false };
        Response { clicked }
    }

    pub fn add_button_horizontal(&mut self, text: &str) -> Response { self.add_button(text) }
}


