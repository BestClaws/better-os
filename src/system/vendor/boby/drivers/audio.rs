use crate::system::hal::audio::{AsyncAudioSink, AudioError};
use core::mem;
use esp_hal::dma::ReadBuffer;
use esp_hal::i2s::master::{Error as I2sError, I2sTx};
use esp_hal::Blocking;

pub struct I2sAudioDriver {
    tx: I2sTx<'static, Blocking>,
    active: bool,
    buffer: Option<SliceReadBuffer<'static>>,
}

impl I2sAudioDriver {
    pub fn new(tx: I2sTx<'static, Blocking>) -> Self {
        Self {
            tx,
            active: false,
            buffer: None,
        }
    }

    fn start_loop_internal(&mut self, data: &'static [i16]) -> Result<(), I2sError> {
        self.buffer = Some(SliceReadBuffer::new(data));
        let buffer_ref = self.buffer.as_ref().unwrap();
        let transfer = self.tx.write_dma_circular(buffer_ref)?;
        mem::forget(transfer);
        self.active = true;
        Ok(())
    }
}

impl AsyncAudioSink for I2sAudioDriver {
    fn play_loop(&mut self, data: &'static [i16]) -> Result<(), AudioError> {
        if self.active {
            return Err(AudioError::AlreadyRunning);
        }

        self.start_loop_internal(data).map_err(AudioError::from)
    }
}

struct SliceReadBuffer<'a> {
    data: &'a [i16],
}

impl<'a> SliceReadBuffer<'a> {
    fn new(data: &'a [i16]) -> Self {
        Self { data }
    }
}

unsafe impl<'a> ReadBuffer for SliceReadBuffer<'a> {
    unsafe fn read_buffer(&self) -> (*const u8, usize) {
        (
            self.data.as_ptr() as *const u8,
            self.data.len() * mem::size_of::<i16>(),
        )
    }
}
