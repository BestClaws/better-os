use crate::system::hal::encoder::Encoder;

pub(crate) struct PlatformDevice<ENCODER: Encoder> {
    pub(crate) encoder: ENCODER,
    
}