use crate::system::input::types::TouchAction;
use crate::system::ui::compositor::animation::TransitionDirection;

/// Tracks edge-originating gestures and converts them into compositor transitions.
///
/// The recognizer is edge-agnostic and can be extended to support additional
/// system gestures (top notification shade, bottom quick settings, etc.).
#[derive(Debug)]
pub struct EdgeSwipeRecognizer {
    tracking: bool,
    fired: bool,
    start_x: i32,
    start_y: i32,
    active_edge: Option<SwipeEdge>,
    frame_width: i32,
    frame_height: i32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SwipeEdge {
    Left,
    Right,
    // Reserved for future use
    Top,
    Bottom,
}

impl EdgeSwipeRecognizer {
    const EDGE_THRESHOLD: i32 = 24;
    const MOVE_EDGE_BUFFER: i32 = 12;
    const MIN_HORIZONTAL_SWIPE: i32 = 60;
    const MAX_VERTICAL_DEVIATION: i32 = 20;

    pub fn new(frame_width: i32, frame_height: i32) -> Self {
        Self {
            tracking: false,
            fired: false,
            start_x: 0,
            start_y: 0,
            active_edge: None,
            frame_width,
            frame_height,
        }
    }

    pub fn calibrate_frame_size(&mut self, width: i32, height: i32) {
        if width > 0 {
            self.frame_width = width;
        }
        if height > 0 {
            self.frame_height = height;
        }
    }

    pub fn process_sample(
        &mut self,
        x: i32,
        y: i32,
        action: TouchAction,
    ) -> (bool, Option<TransitionDirection>) {
        if self.frame_width <= 0 || self.frame_height <= 0 {
            return (false, None);
        }

        match action {
            TouchAction::Down => self.handle_down(x, y),
            TouchAction::Move => self.handle_move(x, y),
            TouchAction::Up => self.handle_up(),
        }
    }

    fn handle_down(&mut self, x: i32, y: i32) -> (bool, Option<TransitionDirection>) {
        self.reset_tracking();
        self.active_edge = self.detect_edge(x, y, Self::EDGE_THRESHOLD);
        if self.active_edge.is_some() {
            self.start_x = x;
            self.start_y = y;
            self.tracking = true;
            return (true, None);
        }
        (false, None)
    }

    fn handle_move(&mut self, x: i32, y: i32) -> (bool, Option<TransitionDirection>) {
        if !self.tracking {
            self.active_edge =
                self.detect_edge(x, y, Self::EDGE_THRESHOLD + Self::MOVE_EDGE_BUFFER);
            if let Some(edge) = self.active_edge {
                self.start_x = x;
                self.start_y = y;
                self.tracking = true;
                return (true, None);
            }
            return (false, None);
        }

        if self.fired {
            return (true, None);
        }

        match self.active_edge {
            Some(SwipeEdge::Left) => self.evaluate_horizontal_swipe(x, y, true),
            Some(SwipeEdge::Right) => self.evaluate_horizontal_swipe(x, y, false),
            Some(SwipeEdge::Top) | Some(SwipeEdge::Bottom) => (true, None),
            None => (false, None),
        }
    }

    fn handle_up(&mut self) -> (bool, Option<TransitionDirection>) {
        let consumed = self.tracking;
        self.reset_tracking();
        (consumed, None)
    }

    fn evaluate_horizontal_swipe(
        &mut self,
        x: i32,
        y: i32,
        from_left: bool,
    ) -> (bool, Option<TransitionDirection>) {
        let dx = x - self.start_x;
        let dy = (y - self.start_y).abs();
        if dy > Self::MAX_VERTICAL_DEVIATION {
            return (true, None);
        }

        if from_left && dx > Self::MIN_HORIZONTAL_SWIPE {
            self.fired = true;
            return (true, Some(TransitionDirection::Next));
        }

        if !from_left && (-dx) > Self::MIN_HORIZONTAL_SWIPE {
            self.fired = true;
            return (true, Some(TransitionDirection::Previous));
        }

        (true, None)
    }

    fn detect_edge(&self, x: i32, y: i32, threshold: i32) -> Option<SwipeEdge> {
        let right_start = (self.frame_width - 1).saturating_sub(threshold);
        let bottom_start = (self.frame_height - 1).saturating_sub(threshold);

        if x <= threshold {
            Some(SwipeEdge::Left)
        } else if x >= right_start {
            Some(SwipeEdge::Right)
        } else if y <= threshold {
            Some(SwipeEdge::Top)
        } else if y >= bottom_start {
            Some(SwipeEdge::Bottom)
        } else {
            None
        }
    }

    fn reset_tracking(&mut self) {
        self.tracking = false;
        self.fired = false;
        self.active_edge = None;
    }
}
