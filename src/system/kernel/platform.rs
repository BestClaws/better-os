use crate::system::hal::display::Display;
use crate::system::hal::encoder::Encoder;

pub(crate) struct PlatformDevice<ENCODER: Encoder, DISPLAY: Display> {
    pub(crate) encoder: Option<ENCODER>,
    pub(crate) display: Option<DISPLAY>,
    
}