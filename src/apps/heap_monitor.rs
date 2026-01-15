use crate::system::app::app_context::AppContext;
use crate::system::ui::drawing_surface::DrawingSurface;
use alloc::{format, string::String};
use embassy_executor::task;
use embassy_time::{Duration, Ticker};
use esp_alloc::{HeapStats, HEAP};
use rust_gfx::color::Rgba8888;
use rust_gfx::primitives::{
    draw_rect,
    label::{draw_label, line_height_for_font, measure_text_with_font, FontId, LabelDsc},
    RectDsc,
};
use rust_gfx::types::{Area, Gradient, OPA_COVER};

#[task]
pub async fn heap_monitor_app(ctx: AppContext) {
    let mut ticker = Ticker::every(Duration::from_secs(1));

    loop {
        if !ctx.is_focused().await {
            ticker.next().await;
            continue;
        }

        let stats = HEAP.stats();
        ctx.draw(|surface| draw_interface(surface, &stats)).await;
        ticker.next().await;
    }
}

fn draw_interface(surface: &mut DrawingSurface, stats: &HeapStats) {
    let width = surface.width() as i32;
    let height = surface.height() as i32;
    if width <= 0 || height <= 0 {
        return;
    }

    let header_font = font_for_role(width, height, FontRole::Header);
    let body_font = font_for_role(width, height, FontRole::Body);
    let header_height = font_height(header_font);
    let body_height = font_height(body_font);

    let mut background = RectDsc::new();
    background.bg_color = Rgba8888::rgba(248, 249, 252, 255);
    background.bg_grad = Gradient::vertical(
        Rgba8888::rgba(255, 255, 255, 255),
        Rgba8888::rgba(232, 234, 240, 255),
    );
    background.bg_opa = OPA_COVER;
    let full_area = Area::new(0, 0, width - 1, height - 1);
    draw_rect(surface, &background, &full_area);

    let header_text = "Heap Monitor";
    let header_width = text_width(header_text, header_font);
    if header_width > 0 {
        let hx = (width - header_width) / 2;
        let hy = 12;
        draw_text(
            surface,
            header_text,
            header_font,
            hx,
            hy,
            Rgba8888::rgba(60, 64, 80, 255),
        );
    }

    let total = stats.size;
    let used = stats.current_usage;
    let free = total.saturating_sub(used);
    let percent = if total == 0 {
        0
    } else {
        ((used as u128 * 100) / total as u128) as u32
    };

    let lines = [
        format!("Total: {}", format_size(total)),
        format!("Used : {}", format_size(used)),
        format!("Free : {}", format_size(free)),
        format!("Usage: {}%", percent),
    ];

    let mut y = 12 + header_height + 8;
    let line_spacing = body_height + 2;

    for line in &lines {
        let text_width = text_width(line, body_font);
        if text_width > 0 {
            let x = (width - text_width) / 2;
            draw_text(
                surface,
                line,
                body_font,
                x,
                y,
                Rgba8888::rgba(86, 92, 110, 255),
            );
        }
        y += line_spacing;
    }

    let bar_width = (width - 24).max(12);
    let bar_height = (font_height(body_font) + 4).min(14).max(10);
    let bar_x1 = (width - bar_width) / 2;
    let bar_y1 = y + 8;
    let bar_area = Area::new(
        bar_x1,
        bar_y1,
        bar_x1 + bar_width - 1,
        bar_y1 + bar_height - 1,
    );

    let mut bar_bg = RectDsc::new();
    bar_bg.bg_color = Rgba8888::rgba(210, 212, 224, 80);
    bar_bg.bg_opa = OPA_COVER;
    bar_bg.radius = bar_height / 2;
    draw_rect(surface, &bar_bg, &bar_area);

    if total > 0 && used > 0 {
        let fill_width = ((used as u128 * bar_width as u128) / total as u128) as i32;
        if fill_width > 0 {
            let fill_area = Area::new(
                bar_x1,
                bar_y1,
                bar_x1 + fill_width - 1,
                bar_y1 + bar_height - 1,
            );
            let mut bar_fill = RectDsc::new();
            bar_fill.bg_color = Rgba8888::rgba(70, 190, 235, 255);
            bar_fill.bg_grad = Gradient::vertical(
                Rgba8888::rgba(70, 190, 235, 255),
                Rgba8888::rgba(40, 120, 200, 255),
            );
            bar_fill.bg_opa = OPA_COVER;
            bar_fill.radius = bar_height / 2;
            draw_rect(surface, &bar_fill, &fill_area);
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
enum FontRole {
    Header,
    Body,
}

fn font_for_role(_width: i32, _height: i32, role: FontRole) -> FontId {
    match role {
        FontRole::Header => FontId::Montserrat12,
        FontRole::Body => FontId::Montserrat8,
    }
}

fn text_width(text: &str, font: FontId) -> i32 {
    if text.is_empty() {
        0
    } else {
        measure_text_with_font(text, 0, font)
    }
}

fn font_height(font: FontId) -> i32 {
    line_height_for_font(font)
}

fn draw_text(surface: &mut DrawingSurface, text: &str, font: FontId, x: i32, y: i32, color: Rgba8888) {
    let width = text_width(text, font);
    if width <= 0 {
        return;
    }
    let height = font_height(font);
    if height <= 0 {
        return;
    }
    let mut label = LabelDsc::new(String::from(text));
    label.font = font;
    label.color = color;
    let area = Area::new(x, y, x + width - 1, y + height - 1);
    draw_label(surface, &label, &area);
}

fn format_size(bytes: usize) -> String {
    const KB: usize = 1024;
    const MB: usize = 1024 * KB;

    if bytes >= MB {
        let whole = bytes / MB;
        let tenths = ((bytes % MB) * 10) / MB;
        if tenths > 0 {
            format!("{}.{} MB", whole, tenths)
        } else {
            format!("{} MB", whole)
        }
    } else {
        let kilobytes = bytes / KB;
        let tenths = ((bytes % KB) * 10) / KB;
        if kilobytes >= 100 || tenths == 0 {
            format!("{} KB", kilobytes)
        } else {
            format!("{}.{} KB", kilobytes, tenths)
        }
    }
}
