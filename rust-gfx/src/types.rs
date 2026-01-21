/// Core types for the graphics library matching LVGL's data structures
use crate::color::Rgba8888;

/// Opacity type matching LVGL's lv_opa_t
pub type Opa = u8;

/// Common opacity values (matching LVGL's LV_OPA_* constants exactly)
pub const OPA_TRANSP: Opa = 0;
pub const OPA_30: Opa = 76;
pub const OPA_40: Opa = 102;
pub const OPA_50: Opa = 127;
pub const OPA_70: Opa = 178;
pub const OPA_COVER: Opa = 255;

/// Matches LVGL's LV_OPA_MIX2(a, b) = (a * b) >> 8.
#[inline]
pub fn opa_mix(a: Opa, b: Opa) -> Opa {
    (((a as u16) * (b as u16)) >> 8) as Opa
}

/// Gradient direction matching LVGL
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum GradDir {
    None,
    Hor,
    Ver,
    Radial,
    Conical,
}

/// Maximum number of gradient stops supported.
pub const MAX_GRADIENT_STOPS: usize = 8;

/// Gradient stop matching LVGL's gradient system
#[derive(Copy, Clone, Debug)]
pub struct GradStop {
    pub color: Rgba8888,
    pub opa: Opa,
    pub frac: u8, // 0-255
}

impl Default for GradStop {
    fn default() -> Self {
        Self {
            color: Rgba8888::WHITE,
            opa: OPA_COVER,
            frac: 0,
        }
    }
}

/// Gradient descriptor matching LVGL
#[derive(Clone, Debug)]
pub struct Gradient {
    pub dir: GradDir,
    pub stops: [GradStop; MAX_GRADIENT_STOPS],
    pub stops_count: usize,
}

impl Gradient {
    pub fn none() -> Self {
        Self {
            dir: GradDir::None,
            stops: Self::build_stops([
                GradStop {
                    color: Rgba8888::WHITE,
                    opa: OPA_COVER,
                    frac: 0,
                },
                GradStop {
                    color: Rgba8888::BLACK,
                    opa: OPA_COVER,
                    frac: 255,
                },
            ]),
            stops_count: 2,
        }
    }

    pub fn horizontal(start_color: Rgba8888, end_color: Rgba8888) -> Self {
        Self {
            dir: GradDir::Hor,
            stops: Self::build_stops([
                GradStop {
                    color: start_color,
                    opa: OPA_COVER,
                    frac: 0,
                },
                GradStop {
                    color: end_color,
                    opa: OPA_COVER,
                    frac: 255,
                },
            ]),
            stops_count: 2,
        }
    }

    pub fn vertical(start_color: Rgba8888, end_color: Rgba8888) -> Self {
        Self {
            dir: GradDir::Ver,
            stops: Self::build_stops([
                GradStop {
                    color: start_color,
                    opa: OPA_COVER,
                    frac: 0,
                },
                GradStop {
                    color: end_color,
                    opa: OPA_COVER,
                    frac: 255,
                },
            ]),
            stops_count: 2,
        }
    }

    pub fn radial(inner_color: Rgba8888, outer_color: Rgba8888) -> Self {
        Self {
            dir: GradDir::Radial,
            stops: Self::build_stops([
                GradStop {
                    color: inner_color,
                    opa: OPA_COVER,
                    frac: 0,
                },
                GradStop {
                    color: outer_color,
                    opa: OPA_COVER,
                    frac: 255,
                },
            ]),
            stops_count: 2,
        }
    }

    pub fn conical(start_color: Rgba8888, end_color: Rgba8888) -> Self {
        Self {
            dir: GradDir::Conical,
            stops: Self::build_stops([
                GradStop {
                    color: start_color,
                    opa: OPA_COVER,
                    frac: 0,
                },
                GradStop {
                    color: end_color,
                    opa: OPA_COVER,
                    frac: 255,
                },
            ]),
            stops_count: 2,
        }
    }

    pub fn from_stops(dir: GradDir, stops: &[GradStop]) -> Self {
        let mut gradient = Self::none();
        gradient.dir = dir;
        let mut idx = 0;
        for stop in stops.iter().take(MAX_GRADIENT_STOPS) {
            gradient.stops[idx] = *stop;
            idx += 1;
        }
        if idx == 0 {
            gradient.stops_count = 1;
            gradient.stops[0] = GradStop::default();
        } else {
            gradient.stops_count = idx;
        }
        gradient
    }

    fn build_stops<const N: usize>(stops: [GradStop; N]) -> [GradStop; MAX_GRADIENT_STOPS] {
        let mut result = [GradStop::default(); MAX_GRADIENT_STOPS];
        let limit = N.min(MAX_GRADIENT_STOPS);
        for i in 0..limit {
            result[i] = stops[i];
        }
        result
    }
}

/// Border side flags matching LVGL
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct BorderSide(pub u8);

impl BorderSide {
    pub const NONE: Self = BorderSide(0x00);
    pub const BOTTOM: Self = BorderSide(0x01);
    pub const TOP: Self = BorderSide(0x02);
    pub const LEFT: Self = BorderSide(0x04);
    pub const RIGHT: Self = BorderSide(0x08);
    pub const FULL: Self = BorderSide(0x0F);

    pub fn has_bottom(self) -> bool {
        self.0 & Self::BOTTOM.0 != 0
    }

    pub fn has_top(self) -> bool {
        self.0 & Self::TOP.0 != 0
    }

    pub fn has_left(self) -> bool {
        self.0 & Self::LEFT.0 != 0
    }

    pub fn has_right(self) -> bool {
        self.0 & Self::RIGHT.0 != 0
    }
}

impl core::ops::BitOr for BorderSide {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self {
        BorderSide(self.0 | rhs.0)
    }
}

/// Text decoration matching LVGL
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum TextDecor {
    None,
    Underline,
    Strikethrough,
}

/// Point with integer coordinates
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Point {
    pub x: i32,
    pub y: i32,
}

impl Point {
    pub fn new(x: i32, y: i32) -> Self {
        Self { x, y }
    }
}

/// Area/rectangle bounds
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Area {
    pub x1: i32,
    pub y1: i32,
    pub x2: i32,
    pub y2: i32,
}

impl Area {
    pub fn new(x1: i32, y1: i32, x2: i32, y2: i32) -> Self {
        Self { x1, y1, x2, y2 }
    }

    pub fn width(&self) -> i32 {
        self.x2 - self.x1 + 1
    }

    pub fn height(&self) -> i32 {
        self.y2 - self.y1 + 1
    }

    pub fn intersect(&self, other: &Area) -> Option<Area> {
        let x1 = self.x1.max(other.x1);
        let y1 = self.y1.max(other.y1);
        let x2 = self.x2.min(other.x2);
        let y2 = self.y2.min(other.y2);

        if x1 <= x2 && y1 <= y2 {
            Some(Area::new(x1, y1, x2, y2))
        } else {
            None
        }
    }
}

/// Radius value, can be LV_RADIUS_CIRCLE
pub const RADIUS_CIRCLE: i32 = 0x7FFF;
