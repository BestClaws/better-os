use crate::system::input::types::{HighLevelEvent, MotionEvent, PointerSample, TouchAction};

/// Normalized touch sample provided by the HAL layer.
pub struct TouchSample {
    pub x: i32,
    pub y: i32,
    pub pressure: i32,
}

/// Stateful processor that turns raw touch samples into debounced high-level events.
pub struct TouchProcessor {
    frame_width: i32,
    frame_height: i32,
    scale_factor: u32,
    coalesce_threshold: i32,
    was_pressed: bool,
    last_pointer: Option<PointerSample>,
}

impl TouchProcessor {
    pub fn new(
        frame_width: u32,
        frame_height: u32,
        scale_factor: u32,
        coalesce_threshold: i32,
    ) -> Self {
        Self {
            frame_width: frame_width as i32,
            frame_height: frame_height as i32,
            scale_factor: scale_factor.max(1),
            coalesce_threshold,
            was_pressed: false,
            last_pointer: None,
        }
    }

    /// Update the logical framebuffer dimensions when the compositor negotiates new metrics.
    pub fn update_dimensions(&mut self, width: u32, height: u32) {
        self.frame_width = width as i32;
        self.frame_height = height as i32;
    }

    pub fn update_scale_factor(&mut self, scale_factor: u32) {
        self.scale_factor = scale_factor.max(1);
    }

    /// Process one raw touch sample. If the sample results in a state transition or a meaningful
    /// move, an appropriate high-level motion event is returned.
    pub fn process_sample(&mut self, sample: TouchSample) -> Option<HighLevelEvent> {
        let pressed = sample.pressure != 0;
        let pointer = self.normalize(sample.x, sample.y);

        let event = match (self.was_pressed, pressed) {
            (false, true) => self.emit_down(pointer),
            (true, true) => self.emit_move(pointer),
            (true, false) => self.emit_up(),
            (false, false) => None,
        };

        if pressed {
            self.last_pointer = pointer.or(self.last_pointer);
        } else if event.is_some() {
            self.last_pointer = None;
        }

        self.was_pressed = pressed;
        event
    }

    /// Returns the last known pointer sample, if any.
    pub fn last_pointer(&self) -> Option<PointerSample> {
        self.last_pointer
    }

    fn emit_down(&mut self, pointer: Option<PointerSample>) -> Option<HighLevelEvent> {
        pointer.map(|p| {
            HighLevelEvent::Motion(MotionEvent {
                action: TouchAction::Down,
                primary_pointer_id: p.id,
                pointers: [Some(p), None],
            })
        })
    }

    fn emit_move(&mut self, pointer: Option<PointerSample>) -> Option<HighLevelEvent> {
        match (pointer, self.last_pointer) {
            (Some(current), Some(previous)) => {
                let delta = (current.x - previous.x).abs() + (current.y - previous.y).abs();
                if delta >= self.coalesce_threshold {
                    self.last_pointer = Some(current);
                    Some(HighLevelEvent::Motion(MotionEvent {
                        action: TouchAction::Move,
                        primary_pointer_id: current.id,
                        pointers: [Some(current), None],
                    }))
                } else {
                    None
                }
            }
            (Some(current), None) => {
                self.last_pointer = Some(current);
                Some(HighLevelEvent::Motion(MotionEvent {
                    action: TouchAction::Move,
                    primary_pointer_id: current.id,
                    pointers: [Some(current), None],
                }))
            }
            _ => None,
        }
    }

    fn emit_up(&mut self) -> Option<HighLevelEvent> {
        self.last_pointer.map(|p| {
            HighLevelEvent::Motion(MotionEvent {
                action: TouchAction::Up,
                primary_pointer_id: p.id,
                pointers: [Some(p), None],
            })
        })
    }

    fn normalize(&self, raw_x: i32, raw_y: i32) -> Option<PointerSample> {
        if self.frame_width <= 0 || self.frame_height <= 0 {
            return None;
        }

        let scaled_x = self.scale_and_clamp(raw_x, self.frame_width);
        let scaled_y = self.scale_and_clamp(raw_y, self.frame_height);

        Some(PointerSample {
            id: 0,
            x: scaled_x,
            y: scaled_y,
        })
    }

    fn scale_and_clamp(&self, raw: i32, bound: i32) -> i32 {
        let clamped = raw.max(0) as u32 / self.scale_factor.max(1);
        let scaled = clamped as i32;
        if bound <= 1 {
            return 0;
        }
        scaled.clamp(0, bound - 1)
    }
}
