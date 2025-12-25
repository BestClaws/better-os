/// Outcome produced by a gesture recognizer after observing a pointer sample.
#[derive(Clone, Copy, Debug)]
pub struct GestureResult<T> {
    pub consumed: bool,
    pub update: Option<T>,
}

impl<T> GestureResult<T> {
    /// Produce a result where the recognizer ignored the sample.
    pub const fn idle() -> Self {
        Self {
            consumed: false,
            update: None,
        }
    }

    /// Produce a result for a consumed sample without emitting an update.
    pub const fn consumed_only() -> Self {
        Self {
            consumed: true,
            update: None,
        }
    }

    /// Helper constructor for arbitrary result variants.
    pub const fn new(consumed: bool, update: Option<T>) -> Self {
        Self { consumed, update }
    }
}

impl<T> From<(bool, Option<T>)> for GestureResult<T> {
    fn from(value: (bool, Option<T>)) -> Self {
        Self {
            consumed: value.0,
            update: value.1,
        }
    }
}
