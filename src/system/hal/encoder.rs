use embedded_hal::digital::InputPin;       // v1.0.0
use embedded_hal_async::digital::Wait;      // v1.0.0
use embassy_time::{Duration, Timer};       // for timing and delays
use async_trait::async_trait;              // to allow async in traits
use alloc::boxed::Box;                     // for heap-allocated boxing
use core::{future::Future, pin::Pin, task::{Context, Poll}}; // core async types

// --------------------------------------------------------------------------------
// Type alias: "BoxedWaitFuture" is just a name to remind us this is:
//  - "Boxed": stored on the heap (via Box<>)
//  - "WaitFuture": it's a Future produced by our wait-for-falling-edge call
// The generic 'E' is the error type returned by that future.
// We give it a name so the long `Pin<Box<dyn Future<...>>>` type is easier to read.

/// BoxedWaitFuture<'a, E> = Pin<Box<dyn Future<Output = Result<(), E>> + Send + 'a>>
/// ^      ^      ^^^^^^         ^^^^^^                         ^^^  ^
/// |      |      |             |                               |    |
/// |      |      |             |                               |    +--- lifetime of the borrow
/// |      |      |             |                               +-------- must implement Send so it's thread-safe
/// |      |      |             +---------------------------------------- dyn Future means unknown type at compile time
/// |      |      +----------------------------------------------- boxed on the heap
/// |      +---------------------------------------------------- Future returning Result<(), E>
/// +--------------------------------------------------------- alias name for clarity

// Alias definition:
type BoxedWaitFuture<'a, E> = Pin<Box<dyn Future<Output = Result<(), E>> + Send + 'a>>;

// --------------------------------------------------------------------------------
// SendFuture wrapper: allows us to take any Future and assert (unsafe!) that it's Send.
// We need this because the HAL's async fn returns a hidden future type that isn't
// automatically marked Send, but we know (by inspection of the HAL code) it's safe.

/// SendFuture<F> wraps a future F and claims it's Send
struct SendFuture<F>(F);

// SAFETY: We (the author) guarantee that the inner future F is indeed safe to send
// across threads/executors. Marking it Send without compiler proof is `unsafe`.
unsafe impl<F> Send for SendFuture<F> {}

// Implement Future for our wrapper by delegating to the inner future's poll()
impl<F: Future> Future for SendFuture<F> {
    type Output = F::Output;
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        // We need to get a pinned mutable reference to the inner F
        // `map_unchecked_mut` is unsafe but okay here because
        // `SendFuture<F>` has the same alignment and size as `F`.
        let inner = unsafe { self.map_unchecked_mut(|s| &mut s.0) };
        inner.poll(cx)
    }
}

// --------------------------------------------------------------------------------
// SendWait trait: extends the HAL's Wait trait to give us a Send-able future

/// Any pin type that implements `Wait` can now also produce a boxed Send future
/// to wait for a falling edge. This method name ends in `_send` to remind us
/// it returns a Send-able future (boxed), unlike the original wait_for_falling_edge().
pub trait SendWait: Wait {
    /// Returns a boxed, Send-able future that completes when a falling edge occurs.
    /// We borrow `self` for `'a` so the future can reference the pin.
    fn wait_for_falling_edge_send<'a>(&'a mut self) -> BoxedWaitFuture<'a, Self::Error>;
}

impl<T> SendWait for T
where
    T: Wait + Send + 'static, // pin type must itself be Send + 'static
{
    fn wait_for_falling_edge_send<'a>(&'a mut self) -> BoxedWaitFuture<'a, T::Error> {
        // Wrap the HAL's own async fn future in SendFuture, then box it
        Box::pin(SendFuture(self.wait_for_falling_edge()))
    }
}

// --------------------------------------------------------------------------------
// Encoder state and error definitions

/// Represents a single step of the encoder, with a placeholder angular speed.
#[derive(Debug)]
pub enum EncoderState {
    Cw(f32),   // clockwise with speed in radians/sec (placeholder)
    Ccw(f32),  // counter-clockwise
}

/// Errors the encoder can produce
#[derive(Debug)]
pub enum EncoderError {
    PinError,      // reading a pin failed
    InvalidState,  // unexpected pin combination
    Timeout,       // waited too long for an edge
}

// --------------------------------------------------------------------------------
// AsyncEncoder trait: defines a single async method `next()`
// The default async_trait (no `?Send`) generates a Send future:
// `fn next<'a>(&'a mut self) -> Pin<Box<dyn Future<Output=...> + Send + 'a>>`

#[async_trait]
pub trait AsyncEncoder: Send {
    /// Waits for and returns the next encoder state (CW or CCW).
    /// Under the hood, it:
    ///  1. waits for a falling edge on pin A (with timeout)
    ///  2. debounces for 1 ms
    ///  3. samples pins A and B
    ///  4. decodes the quadrature direction
    async fn next(&mut self) -> Result<EncoderState, EncoderError>;
}

// --------------------------------------------------------------------------------
// The EncoderDriver struct: holds two pins A and B

pub struct EncoderDriver<P: InputPin + SendWait + Send + 'static> {
    a: P,  // input A
    b: P,  // input B
}

impl<P: InputPin + SendWait + Send + 'static> EncoderDriver<P> {
    /// Create a new EncoderDriver by supplying two GPIO pins.
    /// Pins must support `InputPin` (for is_high()) and `SendWait` (for async edge wait).
    pub fn new(a: P, b: P) -> Self {
        Self { a, b }
    }
}

#[async_trait]
impl<P: InputPin + SendWait + Send + 'static> AsyncEncoder for EncoderDriver<P> {
    async fn next(&mut self) -> Result<EncoderState, EncoderError> {
        // 1) Wait for a falling edge on pin A, with a 100 ms timeout.
        //    We use the Send-able boxed future so we can safely move it if needed.
        if embassy_time::with_timeout(
            Duration::from_millis(100),
            self.a.wait_for_falling_edge_send(),
        ).await.is_err() {
            return Err(EncoderError::Timeout);
        }

        // 2) Debounce: wait 1 ms to avoid spurious bounces on the encoder.
        Timer::after(Duration::from_millis(1)).await;

        // 3) Sample pins A and B. map_err() converts any HAL error into EncoderError::PinError.
        let a_hi = self.a.is_high().map_err(|_| EncoderError::PinError)?;
        let b_hi = self.b.is_high().map_err(|_| EncoderError::PinError)?;

        // 4) Decode quadrature:
        //    - (false, true)  => CCW
        //    - (false, false) => CW
        //    - anything else  => invalid / skipped count
        match (a_hi, b_hi) {
            (false, true)  => Ok(EncoderState::Ccw(1.0)),
            (false, false) => Ok(EncoderState::Cw(1.0)),
            _              => Err(EncoderError::InvalidState),
        }
    }
}
