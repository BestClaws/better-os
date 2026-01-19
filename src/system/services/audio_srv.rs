use crate::system::hal::audio::{AsyncAudioSink, AudioError};
use alloc::boxed::Box;
use core::ptr;
use core::sync::atomic::{AtomicPtr, Ordering};
use defmt::info;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::mutex::Mutex;
use libm::{expf, sinf};

const FRAMES: usize = 512;
const STEREO_SAMPLES: usize = FRAMES * 2;

const SAMPLE_RATE: usize = 48_000;
const BEEP_DURATION_MS: usize = 52;
const BEEP_FRAMES: usize = SAMPLE_RATE * BEEP_DURATION_MS / 1000;
const BEEP_STEREO_SAMPLES: usize = BEEP_FRAMES * 2;

const fn generate_square_wave() -> [i16; STEREO_SAMPLES] {
    let mut data = [0i16; STEREO_SAMPLES];
    let mut frame = 0;
    while frame < FRAMES {
        let level = if (frame / 16) % 2 == 0 {
            10_000
        } else {
            -10_000
        };
        let index = frame * 2;
        data[index] = level;
        data[index + 1] = level;
        frame += 1;
    }
    data
}

fn generate_chime() -> [i16; BEEP_STEREO_SAMPLES] {
    let mut data = [0i16; BEEP_STEREO_SAMPLES];
    let mut frame = 0;
    while frame < BEEP_FRAMES {
        let t = frame as f32 / SAMPLE_RATE as f32;
        let envelope = expf(-5.0 * t);
        let fundamental = sinf(2.0 * core::f32::consts::PI * 880.0 * t);
        let harmonic = sinf(2.0 * core::f32::consts::PI * 1320.0 * t);
        let sample = (fundamental + 0.5 * harmonic) * envelope;
        let scaled = (sample * 18000.0).clamp(-32767.0, 32767.0) as i16;
        let index = frame * 2;
        data[index] = scaled;
        data[index + 1] = scaled;
        frame += 1;
    }
    data
}
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

static AUDIO_LOOP: AudioClip<STEREO_SAMPLES> = AudioClip::new(generate_square_wave);
static MINUTE_BEEP: AudioClip<BEEP_STEREO_SAMPLES> = AudioClip::new(generate_chime);
static AUDIO_DRIVER_PTR: AtomicPtr<()> = AtomicPtr::new(ptr::null_mut());

pub struct AudioService;

impl AudioService {
    pub fn register_driver(
        driver: &'static Mutex<CriticalSectionRawMutex, Box<dyn AsyncAudioSink>>,
    ) -> bool {
        let new_ptr = driver as *const _ as *mut ();
        AUDIO_DRIVER_PTR
            .compare_exchange(ptr::null_mut(), new_ptr, Ordering::SeqCst, Ordering::SeqCst)
            .is_ok()
    }

    pub fn driver() -> Option<&'static Mutex<CriticalSectionRawMutex, Box<dyn AsyncAudioSink>>> {
        let ptr = AUDIO_DRIVER_PTR.load(Ordering::SeqCst);
        if ptr.is_null() {
            None
        } else {
            Some(unsafe {
                &*(ptr as *const Mutex<CriticalSectionRawMutex, Box<dyn AsyncAudioSink>>)
            })
        }
    }

    pub fn loop_clip() -> &'static [i16] {
        AUDIO_LOOP.as_slice()
    }

    pub fn minute_beep_clip() -> &'static [i16] {
        MINUTE_BEEP.as_slice()
    }

    pub async fn play_minute_beep() -> Result<(), AudioError> {
        let driver = Self::driver().ok_or(AudioError::Unavailable)?;
        let mut guard = driver.lock().await;
        info!("audio: minute beep");
        guard.play_once(Self::minute_beep_clip())
    }
}
