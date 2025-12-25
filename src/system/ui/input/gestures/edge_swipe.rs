use core::mem;

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

/// Tunable parameters controlling how aggressively edge swipes are detected and
/// how far the pointer must travel before the compositor reaches 100% progress.
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
            edge_band: 20,
            reentry_slop: 18,
            completion_pixels: 80.0,
            vertical_tolerance: 32,
            progress_epsilon: 0.01,
            commit_fraction: 0.35,
        }
    }
}

#[derive(Clone, Copy, Debug)]
struct FrameMetrics {
    width: i32,
    height: i32,
}

impl FrameMetrics {
    fn new(width: i32, height: i32) -> Self {
        Self {
            width: width.max(0),
            height: height.max(0),
        }
    }

    fn update(&mut self, width: i32, height: i32) {
        if width > 0 {
            self.width = width;
        }
        if height > 0 {
            self.height = height;
        }
    }

    fn is_valid(&self) -> bool {
        self.width > 0 && self.height > 0
    }
}

#[derive(Debug)]
pub struct EdgeSwipeRecognizer {
    config: SwipeConfig,
    metrics: FrameMetrics,
    state: SwipeState,
}

#[derive(Debug)]
enum SwipeState {
    Idle,
    Tracking(GestureSession),
}

#[derive(Debug)]
struct GestureSession {
    start_x: i32,
    start_y: i32,
    direction: TransitionDirection,
    last_progress: f32,
}

impl GestureSession {
    fn new(start_x: i32, start_y: i32, _edge: SwipeEdge, direction: TransitionDirection) -> Self {
        Self {
            start_x,
            start_y,
            direction,
            last_progress: 0.0,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SwipeEdge {
    Left,
    Right,
    Top,
    Bottom,
}

impl EdgeSwipeRecognizer {
    /// Create a new recognizer with default configuration.
    pub fn new(frame_width: i32, frame_height: i32) -> Self {
        Self::with_config(frame_width, frame_height, SwipeConfig::default())
    }

    /// Create a recognizer using an explicit configuration.
    pub fn with_config(frame_width: i32, frame_height: i32, config: SwipeConfig) -> Self {
        Self {
            config,
            metrics: FrameMetrics::new(frame_width, frame_height),
            state: SwipeState::Idle,
        }
    }

    /// Update the gesture configuration at runtime.
    pub fn set_config(&mut self, config: SwipeConfig) {
        self.config = config;
        self.reset();
    }

    /// Update cached framebuffer dimensions.
    pub fn calibrate_frame_size(&mut self, width: i32, height: i32) {
        self.metrics.update(width, height);
    }

    /// Consume a single pointer sample and emit an optional swipe update.
    pub fn process_sample(
        &mut self,
        x: i32,
        y: i32,
        action: TouchAction,
    ) -> (bool, Option<SwipeGestureUpdate>) {
        if !self.metrics.is_valid() {
            return (false, None);
        }

        match action {
            TouchAction::Down => self.handle_down(x, y),
            TouchAction::Move => self.handle_move(x, y),
            TouchAction::Up => self.handle_up(x, y),
        }
    }

    fn handle_down(&mut self, x: i32, y: i32) -> (bool, Option<SwipeGestureUpdate>) {
        self.reset();
        if let Some(edge) = self.detect_edge(x, y, self.config.edge_band) {
            if let Some(direction) = edge.to_direction() {
                self.state = SwipeState::Tracking(GestureSession::new(x, y, edge, direction));
                return (true, None);
            }
        }
        (false, None)
    }

    fn handle_move(&mut self, x: i32, y: i32) -> (bool, Option<SwipeGestureUpdate>) {
        match &mut self.state {
            SwipeState::Idle => {
                let hysteresis = self.config.edge_band + self.config.reentry_slop;
                if let Some(edge) = self.detect_edge(x, y, hysteresis) {
                    if let Some(direction) = edge.to_direction() {
                        self.state =
                            SwipeState::Tracking(GestureSession::new(x, y, edge, direction));
                        return (true, None);
                    }
                }
                (false, None)
            }
            SwipeState::Tracking(session) => {
                if (y - session.start_y).abs() > self.config.vertical_tolerance {
                    let progress = session.last_progress;
                    let direction = session.direction;
                    self.reset();
                    return (
                        true,
                        Some(SwipeGestureUpdate::Cancel {
                            direction,
                            progress,
                        }),
                    );
                }

                let progress = Self::compute_progress(self.config, x, session);
                if (progress - session.last_progress).abs() < self.config.progress_epsilon {
                    return (true, None);
                }
                session.last_progress = progress;
                (
                    true,
                    Some(SwipeGestureUpdate::Preview {
                        direction: session.direction,
                        progress,
                    }),
                )
            }
        }
    }

    fn handle_up(&mut self, x: i32, y: i32) -> (bool, Option<SwipeGestureUpdate>) {
        let mut state = SwipeState::Idle;
        mem::swap(&mut self.state, &mut state);
        match state {
            SwipeState::Idle => (false, None),
            SwipeState::Tracking(mut session) => {
                if (y - session.start_y).abs() > self.config.vertical_tolerance {
                    let direction = session.direction;
                    self.reset();
                    return (
                        true,
                        Some(SwipeGestureUpdate::Cancel {
                            direction,
                            progress: session.last_progress,
                        }),
                    );
                }

                let final_progress = Self::compute_progress(self.config, x, &session);
                session.last_progress = final_progress;
                let direction = session.direction;
                let threshold = self.commit_threshold();
                let update = if final_progress >= threshold {
                    SwipeGestureUpdate::Commit {
                        direction,
                        progress: final_progress,
                    }
                } else {
                    SwipeGestureUpdate::Cancel {
                        direction,
                        progress: final_progress,
                    }
                };
                self.reset();
                (true, Some(update))
            }
        }
    }

    fn detect_edge(&self, x: i32, y: i32, threshold: i32) -> Option<SwipeEdge> {
        if threshold <= 0 {
            return None;
        }
        let right_band = (self.metrics.width - 1).saturating_sub(threshold);
        let bottom_band = (self.metrics.height - 1).saturating_sub(threshold);

        if x <= threshold {
            Some(SwipeEdge::Left)
        } else if x >= right_band {
            Some(SwipeEdge::Right)
        } else if y <= threshold {
            Some(SwipeEdge::Top)
        } else if y >= bottom_band {
            Some(SwipeEdge::Bottom)
        } else {
            None
        }
    }

    fn compute_progress(config: SwipeConfig, x: i32, session: &GestureSession) -> f32 {
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

    fn reset(&mut self) {
        self.state = SwipeState::Idle;
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
