use crate::system::hal::audio::AsyncAudioSink;
use alloc::boxed::Box;
use core::ptr;
use core::sync::atomic::{AtomicPtr, Ordering};
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::mutex::Mutex;

const FRAMES: usize = 512;
const STEREO_SAMPLES: usize = FRAMES * 2;

const fn generate_square_wave() -> [i16; STEREO_SAMPLES] {
    let mut data = [0i16; STEREO_SAMPLES];
    let mut frame = 0;
    while frame < FRAMES {
        let level = if (frame / 16) % 2 == 0 { 10_000 } else { -10_000 };
        let index = frame * 2;
        data[index] = level;
        data[index + 1] = level;
        frame += 1;
    }
    data
}

static AUDIO_LOOP_PTR: AtomicPtr<[i16; STEREO_SAMPLES]> = AtomicPtr::new(ptr::null_mut());

static AUDIO_DRIVER_PTR: AtomicPtr<()> = AtomicPtr::new(ptr::null_mut());

pub fn register_audio_driver(
    driver: &'static Mutex<CriticalSectionRawMutex, Box<dyn AsyncAudioSink>>,
) -> bool {
    let new_ptr = driver as *const _ as *mut ();
    AUDIO_DRIVER_PTR
        .compare_exchange(ptr::null_mut(), new_ptr, Ordering::SeqCst, Ordering::SeqCst)
        .is_ok()
}

pub fn audio_driver(
) -> Option<&'static Mutex<CriticalSectionRawMutex, Box<dyn AsyncAudioSink>>> {
    let ptr = AUDIO_DRIVER_PTR.load(Ordering::SeqCst);
    if ptr.is_null() {
        None
    } else {
        Some(unsafe { &*(ptr as *const Mutex<CriticalSectionRawMutex, Box<dyn AsyncAudioSink>>) })
    }
}

pub fn audio_loop() -> &'static [i16] {
    let ptr = ensure_audio_loop_initialized();
    unsafe { &*ptr }
}

fn ensure_audio_loop_initialized() -> *mut [i16; STEREO_SAMPLES] {
    loop {
        let current = AUDIO_LOOP_PTR.load(Ordering::SeqCst);
        if !current.is_null() {
            return current;
        }

        // Allocate the waveform in DRAM so the I2S DMA engine can access it.
        let boxed = Box::new(generate_square_wave());
        let raw = Box::into_raw(boxed);

        match AUDIO_LOOP_PTR.compare_exchange(
            ptr::null_mut(),
            raw,
            Ordering::SeqCst,
            Ordering::SeqCst,
        ) {
            Ok(_) => return raw,
            Err(existing) => {
                unsafe { drop(Box::from_raw(raw)); }
                if !existing.is_null() {
                    return existing;
                }
            }
        }
    }
}
