use embassy_time::Duration;
use embedded_hal_async::i2c::I2c;

pub trait I2cHelpers<I> where I: I2c {

    fn  read_bit(&mut self, address: u8, register: u8, bit_num: u8, data: &mut [u8], timeout: Duration) -> Result<u8, I::Error>;
    fn read_bits(&mut self, address: u8, register: u8, bit_start: u8, length: u8, data: &mut [u8], timeout: Duration) -> Result<u8, I::Error>;
    fn read_byte(&mut self, address: u8, register: u8, data: &mut [u8], timeout: Duration) -> Result<u8, I::Error>;
    fn read_word(&mut self, address: u8, register: u8, data: &mut [u16], timeout: Duration) -> Result<u8, I::Error>;
    fn read_bytes(&mut self, address: u8, register: u8, length: u8, data: &mut [u8], timeout: Duration) -> Result<u8, I::Error>;
    fn read_words(&mut self, address: u8, register: u8, length: u8, data: &mut [u16], timeout: Duration) -> Result<u8, I::Error>;

    fn write_bit(&mut self, address: u8, register: u8, bit_num: u8, data: &[u8]);
    fn write_bits(&mut self, address: u8, register: u8, bit_start: u8, length: u8, data: &[u8]);
    fn write_byte(&mut self, address: u8, register: u8, data: &[u8]);
    fn write_word(&mut self, address: u8, register: u8, data: &[u16]);
    fn write_bytes(&mut self, address: u8, register: u8, length: u8, data: &mut [u8]);
    fn write_words(&mut self, address: u8, register: u8, length: u8, data: &mut [u16]);
    

}