use crate::libs::gfx::two_d::{Rasterizer, Rect, Point, Size, Rgb565, Rgba8888, LinearGradient, FillStyle, gradient, gradient_vertical, TextRenderer, FONT_8X8};
use crate::libs::gfx::two_d::draw::Draw;
use crate::libs::gfx::two_d::gradients::{fill_rect_rgba};
use crate::libs::gfx::two_d::primitives::{fill_rounded_rect_linear_gradient, draw_rounded_rect_shadow_layers};
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

    /// Clear with a vertical linear gradient background.
    pub fn clear_background_gradient_vertical(&mut self, top: Rgb565, bottom: Rgb565) {
        let rect = Rect::new(Point::new(0, 0), Size::new(self.raster.width(), self.raster.height()));
        let mut d = Draw::new(self.raster);
        d.rect(rect)
            .fill(FillStyle::Linear(gradient_vertical(top, bottom)))
            .draw();
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

        // State
        let hovered = if let Some(p) = self.input.pointer_pos {
            p.x >= rect.top_left.x && p.y >= rect.top_left.y && p.x <= rect.right() && p.y <= rect.bottom()
        } else { false };
        let pressed = hovered && self.input.pointer_down;

        // Draw button
        let border = Stroke { color: Color::GRAY_40, thickness: 1 };
        let corner = CornerRadii { uniform: 6 };
        // subtle shadow first
        // Rounded shadow outside
        draw_rounded_rect_shadow_layers(
            self.raster,
            rect,
            corner.uniform as i32,
            if pressed { 3 } else { 5 },
            Rgb565::from_rgb(32, 32, 32),
            if hovered { 120 } else { 90 },
        );
        // gradient fill for depth (direct raster access)
        // High-contrast green->blue gradient (horizontal)
        let (left_color, right_color) = if pressed {
            (Rgb565::from_rgb(40, 200, 60), Rgb565::from_rgb(20, 110, 220))
        } else if hovered {
            (Rgb565::from_rgb(70, 255, 100), Rgb565::from_rgb(40, 160, 255))
        } else {
            (Rgb565::from_rgb(50, 230, 80), Rgb565::from_rgb(30, 140, 240))
        };
        {
            let mut d = Draw::new(self.raster);
            d.rect(rect)
                .fill(FillStyle::Linear(gradient(left_color, right_color)))
                .draw();
        }
        if hovered && !pressed {
            // stronger highlight for visibility
            fill_rect_rgba(self.raster, rect, Rgba8888::new(255, 255, 255, 28));
        }
        // border stroke in its own short scope to avoid overlapping borrows
        {
            let mut d = Draw::new(self.raster);
            d.rect(rect)
                .stroke(crate::libs::gfx::two_d::draw::stroke(1, Rgb565::from_rgb(255, 255, 255)))
                .draw();
        }

        let text_pos = Point::new(
            rect.top_left.x + pad as i32,
            rect.top_left.y + pad as i32 + if pressed { 1 } else { 0 },
        );
        let mut tr = TextRenderer::new(&FONT_8X8)
            .with_color(Rgb565::from_rgb(255, 255, 255))
            .with_anti_alias(true);
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

    /// Soft shadow under a rect using alpha falloff.
    pub fn shadow(&mut self, rect: Rect, radius: i32, opacity: u8) {
        let grow = radius.max(1) as u32;
        let shadow_rect = Rect::new(Point::new(rect.top_left.x - radius, rect.top_left.y - radius), Size::new(rect.size.width + grow * 2, rect.size.height + grow * 2));
        // simple box blur-ish alpha ring
        for y in shadow_rect.top_left.y..=shadow_rect.bottom() {
            for x in shadow_rect.top_left.x..=shadow_rect.right() {
                // Skip interior of the rect; shadow should be outside only
                if x >= rect.top_left.x && x <= rect.right() && y >= rect.top_left.y && y <= rect.bottom() {
                    continue;
                }
                // distance to nearest point of rect
                let dx = if x < rect.top_left.x { rect.top_left.x - x } else if x > rect.right() { x - rect.right() } else { 0 };
                let dy = if y < rect.top_left.y { rect.top_left.y - y } else if y > rect.bottom() { y - rect.bottom() } else { 0 };
                let d = (dx.max(dy)) as i32;
                if d <= radius {
                    let alpha = (((radius - d) * opacity as i32) / radius).clamp(0, 255) as u8;
                    // dark gray shadow
                    self.raster.blend_pixel(x, y, Rgb565::from_rgb(32, 32, 32), alpha);
                }
            }
        }
    }
}


