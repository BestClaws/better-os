//! Embassy task that owns the audio driver and exposes a chime helper.

use alloc::boxed::Box;
use core::ptr;
use core::sync::atomic::{AtomicPtr, Ordering};

use defmt::{info, warn};
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::channel::Channel;
use embassy_sync::mutex::Mutex;
use libm::{expf, sinf, tanhf};

use crate::system::hal::audio::{AsyncAudioSink, AudioError};

const SAMPLE_RATE: usize = 48_000;
const BEEP_DURATION_MS: usize = 52;
const BEEP_FRAMES: usize = SAMPLE_RATE * BEEP_DURATION_MS / 1000;
const BEEP_STEREO_SAMPLES: usize = BEEP_FRAMES * 2;

type AudioResult = Result<(), AudioError>;

#[derive(Clone, Copy, Debug)]
enum AudioCommand {
    PlayChime,
}

/// Lazily generates and shares static PCM buffers.
struct AudioClip<const N: usize> {
    ptr: AtomicPtr<[i16; N]>,
    generator: fn() -> [i16; N],
}

impl<const N: usize> AudioClip<N> {
    const fn new(generator: fn() -> [i16; N]) -> Self {
        Self {
            ptr: AtomicPtr::new(ptr::null_mut()),
            generator,
        }
    }

    fn ensure(&self) -> *mut [i16; N] {
        loop {
            let current = self.ptr.load(Ordering::SeqCst);
            if !current.is_null() {
                return current;
            }

            let boxed = Box::new((self.generator)());
            let raw = Box::into_raw(boxed);

            match self.ptr.compare_exchange(
                ptr::null_mut(),
                raw,
                Ordering::SeqCst,
                Ordering::SeqCst,
            ) {
                Ok(_) => return raw,
                Err(existing) => {
                    unsafe {
                        drop(Box::from_raw(raw));
                    }
                    if !existing.is_null() {
                        return existing;
                    }
                }
            }
        }
    }

    fn as_slice(&self) -> &'static [i16] {
        let ptr = self.ensure();
        unsafe { &*ptr }
    }
}

fn generate_chime() -> [i16; BEEP_STEREO_SAMPLES] {
    let mut data = [0i16; BEEP_STEREO_SAMPLES];
    let mut frame = 0;

    // Inharmonic partials (metal bar ratios)
    let f1 = 880.0;
    let f2 = 880.0 * 2.76;
    let f3 = 880.0 * 5.40;
    let f4 = 880.0 * 8.93;

    while frame < BEEP_FRAMES {
        let t = frame as f32 / SAMPLE_RATE as f32;

        // Slow bloom, very long decay
        let attack = (t * 8.0).min(1.0);
        let decay = expf(-1.8 * t);
        let envelope = attack * decay;

        let s1 = sinf(2.0 * core::f32::consts::PI * f1 * t);
        let s2 = sinf(2.0 * core::f32::consts::PI * f2 * t);
        let s3 = sinf(2.0 * core::f32::consts::PI * f3 * t);
        let s4 = sinf(2.0 * core::f32::consts::PI * f4 * t);

        // Metallic blend (no strong fundamental)
        let sample = (0.25 * s1 + 0.30 * s2 + 0.25 * s3 + 0.20 * s4) * envelope;

        let scaled = (sample * 26000.0).clamp(-32767.0, 32767.0) as i16;

        let index = frame * 2;
        data[index] = scaled;
        data[index + 1] = scaled;

        frame += 1;
    }

    data
}

// Pre-generated clip and service channels.
static MINUTE_CHIME: AudioClip<BEEP_STEREO_SAMPLES> = AudioClip::new(generate_chime);

static AUDIO_COMMANDS: Channel<CriticalSectionRawMutex, AudioCommand, 4> = Channel::new();
static AUDIO_RESPONSES: Channel<CriticalSectionRawMutex, AudioResult, 1> = Channel::new();

/// Facade API used by apps/tasks.
pub struct AudioService;

impl AudioService {
    /// Request the minute chime and wait for completion or error.
    pub async fn play_minute_beep() -> AudioResult {
        info!("audio: minute beep");
        AUDIO_COMMANDS.sender().send(AudioCommand::PlayChime).await;
        AUDIO_RESPONSES.receiver().receive().await
    }
}

/// Executes a single audio command while holding the driver lock.
async fn handle_command(
    driver: &'static Mutex<CriticalSectionRawMutex, Box<dyn AsyncAudioSink>>,
    command: AudioCommand,
) -> AudioResult {
    match command {
        AudioCommand::PlayChime => {
            let samples = MINUTE_CHIME.as_slice();
            let mut guard = driver.lock().await;
            guard.play_once(samples)
        }
    }
}

#[embassy_executor::task]
pub(crate) async fn audio_service(
    driver: &'static Mutex<CriticalSectionRawMutex, Box<dyn AsyncAudioSink>>,
) {
    info!("audio service starting");

    let request_rx = AUDIO_COMMANDS.receiver();
    let response_tx = AUDIO_RESPONSES.sender();

    loop {
        let command = request_rx.receive().await;
        let result = handle_command(driver, command).await;
        match result {
            Err(err) => {
                warn!("audio command failed: {:?}", err);
                response_tx.send(Err(err)).await;
            }
            Ok(()) => {
                response_tx.send(Ok(())).await;
            }
        }
    }
}

#[embassy_executor::task]
pub(crate) async fn audio_service_unavailable() {
    info!("audio service (stub) starting");
    let request_rx = AUDIO_COMMANDS.receiver();
    let response_tx = AUDIO_RESPONSES.sender();

    loop {
        request_rx.receive().await;
        response_tx.send(Err(AudioError::Unavailable)).await;
    }
}
