use crate::system::input::types::TouchAction;
use crate::system::ui::compositor::animation::TransitionDirection;

/// Outcome emitted by the edge swipe recognizer for the System UI compositor.
#[derive(Clone, Copy, Debug)]
pub enum SwipeGestureUpdate {
    /// Pointer is still active; render the transition at the provided progress (0.0..=1.0).
    Preview {
        direction: TransitionDirection,
        progress: f32,
    },
    /// Pointer lifted and crossed the acceptance threshold; finish the transition.
    Commit {
        direction: TransitionDirection,
        progress: f32,
    },
    /// Pointer lifted (or gesture cancelled) before threshold; revert transition.
    Cancel {
        direction: TransitionDirection,
        progress: f32,
    },
}

/// Tracks edge-originating gestures and converts them into compositor transitions.
///
/// The recognizer is edge-agnostic and can be extended to support additional
/// system gestures (top notification shade, bottom quick settings, etc.).
#[derive(Debug)]
pub struct EdgeSwipeRecognizer {
    tracking: bool,
    start_x: i32,
    start_y: i32,
    active_edge: Option<SwipeEdge>,
    active_direction: Option<TransitionDirection>,
    last_progress: f32,
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
            start_x: 0,
            start_y: 0,
            active_edge: None,
            active_direction: None,
            last_progress: 0.0,
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
    ) -> (bool, Option<SwipeGestureUpdate>) {
        if self.frame_width <= 0 || self.frame_height <= 0 {
            return (false, None);
        }

        match action {
            TouchAction::Down => self.handle_down(x, y),
            TouchAction::Move => self.handle_move(x, y),
            TouchAction::Up => self.handle_up(),
        }
    }

    fn handle_down(&mut self, x: i32, y: i32) -> (bool, Option<SwipeGestureUpdate>) {
        self.reset_tracking();
        self.active_edge = self.detect_edge(x, y, Self::EDGE_THRESHOLD);
        if let Some(edge) = self.active_edge {
            if let Some(direction) = edge.to_direction() {
                self.start_x = x;
                self.start_y = y;
                self.tracking = true;
                self.active_direction = Some(direction);
                return (true, None);
            }
        }
        (false, None)
    }

    fn handle_move(&mut self, x: i32, y: i32) -> (bool, Option<SwipeGestureUpdate>) {
        if !self.tracking {
            self.active_edge =
                self.detect_edge(x, y, Self::EDGE_THRESHOLD + Self::MOVE_EDGE_BUFFER);
            if let Some(edge) = self.active_edge {
                if let Some(direction) = edge.to_direction() {
                    self.start_x = x;
                    self.start_y = y;
                    self.tracking = true;
                    self.active_direction = Some(direction);
                    return (true, None);
                }
            }
            return (false, None);
        }

        let Some(direction) = self.active_direction else {
            return (true, None);
        };

        let dy = (y - self.start_y).abs();
        if dy > Self::MAX_VERTICAL_DEVIATION {
            let progress = self.last_progress;
            let cmd = SwipeGestureUpdate::Cancel {
                direction,
                progress,
            };
            self.reset_tracking();
            return (true, Some(cmd));
        }

        let progress = self.compute_progress(x, direction);
        self.last_progress = progress;
        (
            true,
            Some(SwipeGestureUpdate::Preview {
                direction,
                progress,
            }),
        )
    }

    fn handle_up(&mut self) -> (bool, Option<SwipeGestureUpdate>) {
        if !self.tracking {
            return (false, None);
        }

        let direction = self.active_direction.unwrap_or(TransitionDirection::Next);
        let progress = self.last_progress;
        let threshold = self.commit_threshold();
        let command = if progress >= threshold {
            SwipeGestureUpdate::Commit {
                direction,
                progress,
            }
        } else {
            SwipeGestureUpdate::Cancel {
                direction,
                progress,
            }
        };

        self.reset_tracking();
        (true, Some(command))
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
        self.active_edge = None;
        self.active_direction = None;
        self.last_progress = 0.0;
    }

    fn compute_progress(&self, x: i32, direction: TransitionDirection) -> f32 {
        let width = self.frame_width.max(1) as f32;
        let dx = x - self.start_x;
        let raw = match direction {
            TransitionDirection::Next => dx as f32 / width,
            TransitionDirection::Previous => (-dx) as f32 / width,
        };
        raw.clamp(0.0, 1.0)
    }

    fn commit_threshold(&self) -> f32 {
        let width = self.frame_width.max(1) as f32;
        (Self::MIN_HORIZONTAL_SWIPE as f32 / width).clamp(0.0, 1.0)
    }
}

impl SwipeEdge {
    fn to_direction(self) -> Option<TransitionDirection> {
        match self {
            SwipeEdge::Left => Some(TransitionDirection::Next),
            SwipeEdge::Right => Some(TransitionDirection::Previous),
            SwipeEdge::Top | SwipeEdge::Bottom => None,
        }
    }
}
