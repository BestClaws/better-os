/// Simple RGB color used by UI style.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl Color {
    pub const fn rgb(r: u8, g: u8, b: u8) -> Self { Self { r, g, b } }
    pub const BLACK: Self = Self { r: 0, g: 0, b: 0 };
    pub const WHITE: Self = Self { r: 255, g: 255, b: 255 };
    pub const GRAY_20: Self = Self { r: 51, g: 51, b: 51 };
    pub const GRAY_40: Self = Self { r: 102, g: 102, b: 102 };
    pub const GRAY_80: Self = Self { r: 204, g: 204, b: 204 };
    pub const BLUE: Self = Self { r: 30, g: 144, b: 255 };
    pub const RED: Self = Self { r: 220, g: 20, b: 60 };
    pub const GREEN: Self = Self { r: 34, g: 139, b: 34 };
}

#[derive(Clone, Copy, Debug)]
pub struct Stroke {
    pub color: Color,
    pub thickness: u8,
}

impl Stroke {
    pub const fn new(color: Color, thickness: u8) -> Self { Self { color, thickness } }
}

#[derive(Clone, Copy, Debug)]
pub struct Fill {
    pub color: Color,
}

impl Fill { pub const fn new(color: Color) -> Self { Self { color } } }

#[derive(Clone, Copy, Debug, Default)]
pub struct CornerRadii { pub uniform: u8 }

#[derive(Clone, Copy, Debug)]
pub struct TextStyle {
    pub color: Color,
}

impl Default for TextStyle {
    fn default() -> Self { Self { color: Color::WHITE } }
}

#[derive(Clone, Copy, Debug)]
pub struct Theme {
    pub background: Fill,
    pub surface: Fill,
    pub on_surface: Color,
    pub primary: Color,
    pub on_primary: Color,
    pub border: Stroke,
}

impl Default for Theme {
    fn default() -> Self {
        Self {
            background: Fill::new(Color::BLACK),
            surface: Fill::new(Color::GRAY_20),
            on_surface: Color::GRAY_80,
            primary: Color::BLUE,
            on_primary: Color::WHITE,
            border: Stroke::new(Color::GRAY_40, 1),
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Palette;

impl Palette {
    pub const fn shadow() -> Color { Color::rgb(0, 0, 0) }
    pub const fn highlight() -> Color { Color::rgb(255, 255, 255) }
}


