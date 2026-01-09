use crate::system::input::types::TouchAction;
use crate::system::ui::compositor::animation::TransitionDirection;
use crate::system::ui::input::gestures::core::{FrameSpace, ScreenEdge};
use crate::system::ui::input::gestures::{GestureResult, PointerEvent, PointerGesture};

/// Tunable parameters controlling how aggressively edge swipes are detected.
#[derive(Clone, Copy, Debug)]
pub struct SwipeConfig {
    /// Logical pixels near the edge that are reserved for swipe initiation.
    pub edge_band: i32,
    /// Additional hysteresis applied after a gesture starts, keeping it latched
    /// to the edge even if the finger strays slightly inward.
    pub reentry_slop: i32,
    /// Horizontal distance (in logical pixels) required to report 100% progress.
    pub completion_pixels: f32,
    /// Maximum vertical drift tolerated before the gesture is cancelled.
    pub vertical_tolerance: i32,
    /// Minimum delta required before a new preview update is emitted.
    pub progress_epsilon: f32,
    /// Progress fraction required to convert a release into a commit.
    pub commit_fraction: f32,
}

impl Default for SwipeConfig {
    fn default() -> Self {
        Self {
            edge_band: 12,           // Reduced from 20 for narrower display (410px physical)
            reentry_slop: 10,        // Reduced from 18 proportionally
            completion_pixels: 60.0, // Reduced from 80.0 for narrower display
            vertical_tolerance: 40,  // Increased from 32 for taller display (502px physical)
            progress_epsilon: 0.01,
            commit_fraction: 0.35,
        }
    }
}

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

/// Swipe-from-edge gesture recognizer powering window transitions in the System UI.
pub struct EdgeSwipeRecognizer {
    config: SwipeConfig,
    frame: FrameSpace,
    state: RecognizerState,
}

impl EdgeSwipeRecognizer {
    pub fn new(frame_width: i32, frame_height: i32) -> Self {
        Self::with_config(frame_width, frame_height, SwipeConfig::default())
    }

    pub fn with_config(frame_width: i32, frame_height: i32, config: SwipeConfig) -> Self {
        Self {
            config,
            frame: FrameSpace::new(frame_width, frame_height),
            state: RecognizerState::Idle,
        }
    }

    pub fn set_config(&mut self, config: SwipeConfig) {
        self.config = config;
        self.state = RecognizerState::Idle;
    }

    pub fn calibrate_frame_size(&mut self, width: i32, height: i32) {
        self.frame.update(width, height);
    }

    fn handle_down(&mut self, event: PointerEvent) -> GestureResult<SwipeGestureUpdate> {
        self.state = RecognizerState::Idle;
        if self.try_begin_session(event.pointer.x, event.pointer.y, self.config.edge_band) {
            GestureResult::consumed_only()
        } else {
            GestureResult::idle()
        }
    }

    fn handle_move(&mut self, event: PointerEvent) -> GestureResult<SwipeGestureUpdate> {
        if let RecognizerState::Tracking(ref mut session) = self.state {
            if session.vertical_delta(event.pointer.y) > self.config.vertical_tolerance {
                let direction = session.direction;
                let progress = session.progress;
                self.state = RecognizerState::Idle;
                return GestureResult::new(
                    true,
                    Some(SwipeGestureUpdate::Cancel {
                        direction,
                        progress,
                    }),
                );
            }

            let progress = Self::compute_progress(self.config, session, event.pointer.x);
            if (progress - session.progress).abs() < self.config.progress_epsilon {
                return GestureResult::consumed_only();
            }

            session.progress = progress;
            return GestureResult::new(
                true,
                Some(SwipeGestureUpdate::Preview {
                    direction: session.direction,
                    progress,
                }),
            );
        }

        let hysteresis = self.config.edge_band + self.config.reentry_slop;
        if self.try_begin_session(event.pointer.x, event.pointer.y, hysteresis) {
            GestureResult::consumed_only()
        } else {
            GestureResult::idle()
        }
    }

    fn handle_up(&mut self, event: PointerEvent) -> GestureResult<SwipeGestureUpdate> {
        let RecognizerState::Tracking(mut session) =
            core::mem::replace(&mut self.state, RecognizerState::Idle)
        else {
            return GestureResult::idle();
        };

        if session.vertical_delta(event.pointer.y) > self.config.vertical_tolerance {
            return GestureResult::new(
                true,
                Some(SwipeGestureUpdate::Cancel {
                    direction: session.direction,
                    progress: session.progress,
                }),
            );
        }

        let final_progress = Self::compute_progress(self.config, &session, event.pointer.x);
        session.progress = final_progress;
        let threshold = self.commit_threshold();
        let update = if final_progress >= threshold {
            SwipeGestureUpdate::Commit {
                direction: session.direction,
                progress: final_progress,
            }
        } else {
            SwipeGestureUpdate::Cancel {
                direction: session.direction,
                progress: final_progress,
            }
        };
        GestureResult::new(true, Some(update))
    }

    fn try_begin_session(&mut self, x: i32, y: i32, threshold: i32) -> bool {
        if let Some(edge) = self.detect_edge(x, y, threshold) {
            if let Some(direction) = edge_to_direction(edge) {
                self.state = RecognizerState::Tracking(SwipeSession::new(x, y, direction));
                return true;
            }
        }
        false
    }

    fn detect_edge(&self, x: i32, y: i32, threshold: i32) -> Option<ScreenEdge> {
        if threshold <= 0 {
            return None;
        }

        if x <= threshold {
            Some(ScreenEdge::Left)
        } else if x >= self.frame.right_band(threshold) {
            Some(ScreenEdge::Right)
        } else if y <= threshold {
            Some(ScreenEdge::Top)
        } else if y >= self.frame.bottom_band(threshold) {
            Some(ScreenEdge::Bottom)
        } else {
            None
        }
    }

    fn compute_progress(config: SwipeConfig, session: &SwipeSession, x: i32) -> f32 {
        let travelled = match session.direction {
            TransitionDirection::Next => (x - session.start_x) as f32,
            TransitionDirection::Previous => (session.start_x - x) as f32,
        };
        let normalized = travelled / config.completion_pixels.max(1.0);
        normalized.clamp(0.0, 1.0)
    }

    fn commit_threshold(&self) -> f32 {
        self.config.commit_fraction.clamp(0.0, 1.0)
    }
}

impl PointerGesture for EdgeSwipeRecognizer {
    type Update = SwipeGestureUpdate;

    fn name(&self) -> &'static str {
        "edge_swipe"
    }

    fn observe(&mut self, event: PointerEvent) -> GestureResult<Self::Update> {
        if !self.frame.is_valid() {
            return GestureResult::idle();
        }

        match event.action {
            TouchAction::Down => self.handle_down(event),
            TouchAction::Move => self.handle_move(event),
            TouchAction::Up => self.handle_up(event),
        }
    }

    fn reset(&mut self) {
        self.state = RecognizerState::Idle;
    }

    fn is_tracking(&self) -> bool {
        matches!(self.state, RecognizerState::Tracking(_))
    }
}

#[derive(Debug)]
enum RecognizerState {
    Idle,
    Tracking(SwipeSession),
}

#[derive(Debug)]
struct SwipeSession {
    start_x: i32,
    start_y: i32,
    direction: TransitionDirection,
    progress: f32,
}

impl SwipeSession {
    fn new(start_x: i32, start_y: i32, direction: TransitionDirection) -> Self {
        Self {
            start_x,
            start_y,
            direction,
            progress: 0.0,
        }
    }

    fn vertical_delta(&self, y: i32) -> i32 {
        (y - self.start_y).abs()
    }
}

fn edge_to_direction(edge: ScreenEdge) -> Option<TransitionDirection> {
    match edge {
        ScreenEdge::Left => Some(TransitionDirection::Next),
        ScreenEdge::Right => Some(TransitionDirection::Previous),
        ScreenEdge::Top | ScreenEdge::Bottom => None,
    }
}
