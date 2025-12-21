
#[derive(Copy, Clone)]
pub enum Fill {
    Solid(u16, u8),
    RadialGradient {
        inner_color: u16,
        inner_alpha: u8,
        outer_color: u16,
        outer_alpha: u8,
    },
    LinearGradientH {
        start_color: u16,
        start_alpha: u8,
        end_color: u16,
        end_alpha: u8,
    },
    LinearGradientV {
        start_color: u16,
        start_alpha: u8,
        end_color: u16,
        end_alpha: u8,
    },
}