use rust_gfx::color::Rgba8888;
use rust_gfx::primitives::label::{line_height_for_font, FontId};

use crate::system::ui::drawing_surface::DrawingSurface;

#[derive(Clone, Copy, Debug)]
pub struct StylePalette {
    pub background: Rgba8888,
    pub container: Rgba8888,
    pub container_alt: Rgba8888,
    pub text_primary: Rgba8888,
    pub text_secondary: Rgba8888,
    pub text_muted: Rgba8888,
    pub accent_yellow: Rgba8888,
    pub accent_dark: Rgba8888,
    pub accent_gray: Rgba8888,
    pub accent_light: Rgba8888,
    pub stat_gradient_start: Rgba8888,
    pub stat_gradient_end: Rgba8888,
    pub outline: Rgba8888,
    pub shadow: Rgba8888,
}

impl StylePalette {
    pub const fn arknights() -> Self {
        Self {
            background: Rgba8888::rgba(232, 232, 232, 255),
            container: Rgba8888::rgba(255, 255, 255, 240),
            container_alt: Rgba8888::rgba(250, 250, 250, 255),
            text_primary: Rgba8888::rgba(44, 44, 44, 255),
            text_secondary: Rgba8888::rgba(102, 102, 102, 255),
            text_muted: Rgba8888::rgba(136, 136, 136, 255),
            accent_yellow: Rgba8888::rgba(247, 255, 0, 255),
            accent_dark: Rgba8888::rgba(44, 44, 44, 255),
            accent_gray: Rgba8888::rgba(102, 102, 102, 255),
            accent_light: Rgba8888::rgba(216, 216, 216, 255),
            stat_gradient_start: Rgba8888::rgba(44, 44, 44, 255),
            stat_gradient_end: Rgba8888::rgba(68, 68, 68, 255),
            outline: Rgba8888::rgba(210, 210, 210, 255),
            shadow: Rgba8888::rgba(120, 120, 120, 40),
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct StyleFonts {
    pub status: FontId,
    pub title: FontId,
    pub body: FontId,
    pub small: FontId,
    pub display: FontId,
}

impl StyleFonts {
    pub fn for_surface(width: i32, height: i32) -> Self {
        // Use smaller fonts on constrained displays.
        let narrow = width <= 120;
        let short = height <= 140;
        if narrow || short {
            Self {
                status: FontId::Montserrat8,
                title: FontId::Montserrat10,
                body: FontId::Montserrat8,
                small: FontId::Montserrat8,
                display: FontId::Montserrat24,
            }
        } else if width <= 180 || height <= 200 {
            Self {
                status: FontId::Montserrat10,
                title: FontId::Montserrat12,
                body: FontId::Montserrat10,
                small: FontId::Montserrat8,
                display: FontId::Montserrat28,
            }
        } else {
            Self {
                status: FontId::Montserrat12,
                title: FontId::Montserrat14,
                body: FontId::Montserrat12,
                small: FontId::Montserrat10,
                display: FontId::Montserrat32,
            }
        }
    }

    pub fn line_height_status(self) -> i32 {
        line_height_for_font(self.status)
    }

    pub fn line_height_title(self) -> i32 {
        line_height_for_font(self.title)
    }

    pub fn line_height_body(self) -> i32 {
        line_height_for_font(self.body)
    }

    pub fn line_height_small(self) -> i32 {
        line_height_for_font(self.small)
    }

    pub fn line_height_display(self) -> i32 {
        line_height_for_font(self.display)
    }
}

#[derive(Clone, Copy, Debug)]
pub struct StyleMetrics {
    pub width: i32,
    pub height: i32,
    pub outer_padding: i32,
    pub section_spacing: i32,
    pub section_padding: i32,
    pub section_radius: i32,
    pub quick_gap: i32,
    pub button_height: i32,
}

impl StyleMetrics {
    pub fn from_surface(surface: &DrawingSurface) -> Self {
        let width = surface.width() as i32;
        let height = surface.height() as i32;
        Self::from_dimensions(width, height)
    }

    pub fn from_dimensions(width: i32, height: i32) -> Self {
        let outer_padding = clamp(width / 18, 4, 10);
        let section_spacing = clamp(height / 48, 4, 12);
        let section_padding = clamp(width / 22, 4, 12);
        let section_radius = clamp(width / 14, 4, 12);
        let quick_gap = clamp(width / 28, 2, 8);
        let button_height = clamp(height / 8, 16, 28);

        Self {
            width,
            height,
            outer_padding,
            section_spacing,
            section_padding,
            section_radius,
            quick_gap,
            button_height,
        }
    }

    pub fn content_x(&self) -> i32 {
        self.outer_padding
    }

    pub fn content_width(&self) -> i32 {
        (self.width - self.outer_padding * 2).max(0)
    }
}

const fn clamp(value: i32, min: i32, max: i32) -> i32 {
    if value < min {
        min
    } else if value > max {
        max
    } else {
        value
    }
}
