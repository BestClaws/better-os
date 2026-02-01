/// Public Interface Color Format
/// all inputs are given in this format
#[derive(Copy, Clone)]
pub struct Color(u32);

impl Color {
    pub fn rgba(r: u8, g: u8, b: u8, a: u8) -> Self {
        Color(
            (r as u32) << 24 |
                (g as u32) << 16 |
                (b as u32) << 8 |
                (a as u32))
    }


    pub fn r(self) -> u8 {
        (self.0 >> 24) as u8
    }

    pub fn g(self) -> u8 {
        (self.0 >> 16) as u8
    }

    pub fn b(self) -> u8 {
        (self.0 >> 8) as u8
    }
    pub fn a(self) -> u8 {
        self.0 as u8
    }

}