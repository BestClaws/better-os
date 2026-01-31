/// Tracks the logical framebuffer dimensions used for gesture heuristics.
#[derive(Clone, Copy, Debug)]
pub struct FrameSpace {
    width: i32,
    height: i32,
}

impl FrameSpace {
    pub fn new(width: i32, height: i32) -> Self {
        Self {
            width: width.max(0),
            height: height.max(0),
        }
    }

    pub fn update(&mut self, width: i32, height: i32) {
        if width > 0 {
            self.width = width;
        }
        if height > 0 {
            self.height = height;
        }
    }

    pub fn width(&self) -> i32 {
        self.width
    }

    pub fn height(&self) -> i32 {
        self.height
    }

    pub fn is_valid(&self) -> bool {
        self.width > 0 && self.height > 0
    }

    pub fn right_band(&self, threshold: i32) -> i32 {
        (self.width - 1).saturating_sub(threshold)
    }

    pub fn bottom_band(&self, threshold: i32) -> i32 {
        (self.height - 1).saturating_sub(threshold)
    }
}
