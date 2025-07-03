use alloc::boxed::Box;
use crate::system::hal::encoder::Encoder;

pub(crate) struct PlatformDevice<A, B> {
    pub(crate) encoder: Box<dyn Encoder>
}