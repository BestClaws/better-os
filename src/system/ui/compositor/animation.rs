use defmt::Format;
use libm::sqrtf;
use micromath::F32Ext;

#[derive(Clone, Copy)]
pub struct AnimationConfig {
    pub steps: usize,
    pub frame_delay_ms: u64,
    pub easing_fn: EasingFn,
}

impl Default for AnimationConfig {
    fn default() -> Self {
        Self {
            steps: 8,
            frame_delay_ms: 25,
            easing_fn: ease_in_out_cubic,
        }
    }
}

#[derive(Clone, Copy, Debug, Format)]
pub enum TransitionDirection {
    Previous,
    Next,
}

#[derive(Clone, Copy, Debug)]
pub struct AnimationFrame {
    pub source_x: i32,
    pub source_y: i32,
    pub target_x: i32,
    pub target_y: i32,
    pub scale: f32,
}

pub trait WindowAnimation {
    fn animate_frame(
        &self,
        progress: f32,
        direction: TransitionDirection,
        screen_width: u32,
    ) -> AnimationFrame;
    fn name(&self) -> &'static str;
}

#[derive(Default)]
pub struct SlideZoomAnimation;

impl WindowAnimation for SlideZoomAnimation {
    fn animate_frame(
        &self,
        progress: f32,
        direction: TransitionDirection,
        screen_width: u32,
    ) -> AnimationFrame {
        let width = screen_width as i32;
        let offset = (progress * width as f32) as i32;
        let zoom_factor = if progress < 0.5 {
            1.0 - (progress * 0.1)
        } else {
            0.95 + ((progress - 0.5) * 0.1)
        };
        match direction {
            TransitionDirection::Previous => AnimationFrame {
                source_x: -offset,
                source_y: 0,
                target_x: -offset + width,
                target_y: 0,
                scale: zoom_factor,
            },
            TransitionDirection::Next => AnimationFrame {
                source_x: offset,
                source_y: 0,
                target_x: offset - width,
                target_y: 0,
                scale: zoom_factor,
            },
        }
    }
    fn name(&self) -> &'static str {
        "SlideZoom"
    }
}

#[derive(Default)]
pub struct FadeAnimation;

impl WindowAnimation for FadeAnimation {
    fn animate_frame(
        &self,
        _progress: f32,
        _direction: TransitionDirection,
        _screen_width: u32,
    ) -> AnimationFrame {
        AnimationFrame {
            source_x: 0,
            source_y: 0,
            target_x: 0,
            target_y: 0,
            scale: 1.0,
        }
    }
    fn name(&self) -> &'static str {
        "Fade"
    }
}

pub type EasingFn = fn(f32) -> f32;
pub fn ease_in_out_cubic(t: f32) -> f32 {
    if t < 0.5 {
        4.0 * t * t * t
    } else {
        let shifted = 2.0 * t - 2.0;
        1.0 + shifted * shifted * shifted / 2.0
    }
}
pub fn ease_in_out_circular(t: f32) -> f32 {
    if t < 0.5 {
        0.5 * (1.0 - sqrtf(1.0 - 4.0 * t * t))
    } else {
        0.5 * (sqrtf(1.0 - (2.0 * t - 2.0).powf(2.0)) + 1.0)
    }
}
pub fn ease_out_bounce(t: f32) -> f32 {
    const N1: f32 = 7.5625;
    const D1: f32 = 2.75;
    if t < 1.0 / D1 {
        N1 * t * t
    } else if t < 2.0 / D1 {
        let shifted = t - 1.5 / D1;
        N1 * shifted * shifted + 0.75
    } else if t < 2.5 / D1 {
        let shifted = t - 2.25 / D1;
        N1 * shifted * shifted + 0.9375
    } else {
        let shifted = t - 2.625 / D1;
        N1 * shifted * shifted + 0.984375
    }
}
