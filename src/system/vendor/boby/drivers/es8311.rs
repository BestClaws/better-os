use core::fmt;
use defmt::info;
use embedded_hal::delay::DelayNs;
use embedded_hal::digital::OutputPin;
use embedded_hal::i2c::{I2c, SevenBitAddress};

const ES8311_I2C_ADDR: SevenBitAddress = 0x18;
const ES8311_RESET_REG00: u8 = 0x00;
const ES8311_CLK_MANAGER_REG01: u8 = 0x01;
const ES8311_CLK_MANAGER_REG02: u8 = 0x02;
const ES8311_CLK_MANAGER_REG03: u8 = 0x03;
const ES8311_CLK_MANAGER_REG04: u8 = 0x04;
const ES8311_CLK_MANAGER_REG05: u8 = 0x05;
const ES8311_CLK_MANAGER_REG06: u8 = 0x06;
const ES8311_CLK_MANAGER_REG07: u8 = 0x07;
const ES8311_CLK_MANAGER_REG08: u8 = 0x08;
const ES8311_SDPIN_REG09: u8 = 0x09;
const ES8311_SDPOUT_REG0A: u8 = 0x0A;
const ES8311_SYSTEM_REG0B: u8 = 0x0B;
const ES8311_SYSTEM_REG0C: u8 = 0x0C;
const ES8311_SYSTEM_REG0D: u8 = 0x0D;
const ES8311_SYSTEM_REG0E: u8 = 0x0E;
const ES8311_SYSTEM_REG10: u8 = 0x10;
const ES8311_SYSTEM_REG11: u8 = 0x11;
const ES8311_SYSTEM_REG12: u8 = 0x12;
const ES8311_SYSTEM_REG13: u8 = 0x13;
const ES8311_SYSTEM_REG14: u8 = 0x14;
const ES8311_ADC_REG15: u8 = 0x15;
const ES8311_ADC_REG16: u8 = 0x16;
const ES8311_ADC_REG17: u8 = 0x17;
const ES8311_ADC_REG1B: u8 = 0x1B;
const ES8311_ADC_REG1C: u8 = 0x1C;
const ES8311_DAC_REG31: u8 = 0x31;
const ES8311_DAC_REG32: u8 = 0x32;
const ES8311_DAC_REG37: u8 = 0x37;
const ES8311_GPIO_REG44: u8 = 0x44;
const ES8311_GP_REG45: u8 = 0x45;

#[derive(Clone, Copy)]
struct ClockConfig {
    pre_div: u8,
    pre_multi: u8,
    adc_div: u8,
    dac_div: u8,
    fs_mode: u8,
    lrck_h: u8,
    lrck_l: u8,
    bclk_div: u8,
    adc_osr: u8,
    dac_osr: u8,
}

const CLOCK_48K_EXTERNAL_MCLK: ClockConfig = ClockConfig {
    pre_div: 1,
    pre_multi: 1,
    adc_div: 1,
    dac_div: 1,
    fs_mode: 0,
    lrck_h: 0x00,
    lrck_l: 0xFF,
    bclk_div: 0x04,
    adc_osr: 0x10,
    dac_osr: 0x10,
};

// Derives dig_mclk from the host's BCLK when no dedicated MCLK pin is wired.
const CLOCK_48K_FROM_BCLK: ClockConfig = ClockConfig {
    pre_div: 1,
    pre_multi: 8,
    adc_div: 1,
    dac_div: 1,
    fs_mode: 0,
    lrck_h: 0x00,
    lrck_l: 0xFF,
    bclk_div: 0x04,
    adc_osr: 0x10,
    dac_osr: 0x10,
};

#[derive(Debug)]
pub enum Error<I2cError, PinError> {
    I2c(I2cError),
    Pin(PinError),
    UnsupportedSampleRate,
}

impl<I2cError, PinError> fmt::Display for Error<I2cError, PinError>
where
    I2cError: fmt::Debug,
    PinError: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::I2c(err) => write!(f, "I2C error: {:?}", err),
            Error::Pin(err) => write!(f, "GPIO error: {:?}", err),
            Error::UnsupportedSampleRate => write!(f, "unsupported sample rate"),
        }
    }
}

pub fn init_codec<I2C, PA, DELAY>(
    i2c: &mut I2C,
    pa_enable: &mut PA,
    delay: &mut DELAY,
    sample_rate_hz: u32,
    use_external_mclk: bool,
) -> Result<(), Error<I2C::Error, PA::Error>>
where
    I2C: I2c<SevenBitAddress>,
    PA: OutputPin,
    DELAY: DelayNs,
{
    pa_enable.set_low().map_err(Error::Pin)?;
    delay.delay_ms(1);

    write_reg(i2c, ES8311_GPIO_REG44, 0x08)?;
    write_reg(i2c, ES8311_GPIO_REG44, 0x08)?;
    write_reg(i2c, ES8311_CLK_MANAGER_REG01, 0x30)?;
    write_reg(i2c, ES8311_CLK_MANAGER_REG02, 0x00)?;
    write_reg(i2c, ES8311_CLK_MANAGER_REG03, 0x10)?;
    write_reg(i2c, ES8311_ADC_REG16, 0x24)?;
    write_reg(i2c, ES8311_CLK_MANAGER_REG04, 0x10)?;
    write_reg(i2c, ES8311_CLK_MANAGER_REG05, 0x00)?;
    write_reg(i2c, ES8311_SYSTEM_REG0B, 0x00)?;
    write_reg(i2c, ES8311_SYSTEM_REG0C, 0x00)?;
    write_reg(i2c, ES8311_SYSTEM_REG10, 0x1F)?;
    write_reg(i2c, ES8311_SYSTEM_REG11, 0x7F)?;
    write_reg(i2c, ES8311_RESET_REG00, 0x80)?;
    delay.delay_ms(5);

    let mut reg0 = read_reg(i2c, ES8311_RESET_REG00)?;
    reg0 &= !0x40; // force slave mode
    write_reg(i2c, ES8311_RESET_REG00, reg0)?;

    write_reg(i2c, ES8311_CLK_MANAGER_REG01, 0x3F)?;

    let mut reg1 = read_reg(i2c, ES8311_CLK_MANAGER_REG01)?;
    if use_external_mclk {
        reg1 &= !0x80;
    } else {
        reg1 |= 0x80;
    }
    write_reg(i2c, ES8311_CLK_MANAGER_REG01, reg1)?;

    let clock = match sample_rate_hz {
        48_000 => {
            if use_external_mclk {
                CLOCK_48K_EXTERNAL_MCLK
            } else {
                CLOCK_48K_FROM_BCLK
            }
        }
        _ => return Err(Error::UnsupportedSampleRate),
    };

    configure_clock(i2c, &clock)?;

    let mut reg1 = read_reg(i2c, ES8311_CLK_MANAGER_REG01)?;
    reg1 &= !0x40; // do not invert mclk
    write_reg(i2c, ES8311_CLK_MANAGER_REG01, reg1)?;

    let mut reg6 = read_reg(i2c, ES8311_CLK_MANAGER_REG06)?;
    reg6 &= !0x20; // do not invert bclk
    write_reg(i2c, ES8311_CLK_MANAGER_REG06, reg6)?;

    write_reg(i2c, ES8311_SYSTEM_REG13, 0x10)?;
    write_reg(i2c, ES8311_ADC_REG1B, 0x0A)?;
    write_reg(i2c, ES8311_ADC_REG1C, 0x6A)?;

    write_reg(i2c, ES8311_SDPIN_REG09, 0x0C)?;
    write_reg(i2c, ES8311_SDPOUT_REG0A, 0x0C)?;

    write_reg(i2c, ES8311_ADC_REG17, 0xBF)?;
    write_reg(i2c, ES8311_SYSTEM_REG0E, 0x02)?;
    write_reg(i2c, ES8311_SYSTEM_REG12, 0x00)?;
    write_reg(i2c, ES8311_SYSTEM_REG14, 0x1A)?;
    write_reg(i2c, ES8311_SYSTEM_REG0D, 0x01)?;
    write_reg(i2c, ES8311_ADC_REG15, 0x40)?;
    write_reg(i2c, ES8311_DAC_REG37, 0x08)?;
    write_reg(i2c, ES8311_GP_REG45, 0x00)?;
    write_reg(i2c, ES8311_GPIO_REG44, 0x58)?;
    write_reg(i2c, ES8311_DAC_REG31, 0x00)?;
    write_reg(i2c, ES8311_DAC_REG32, 0xBF)?;

    delay.delay_ms(1);
    pa_enable.set_high().map_err(Error::Pin)?;
    info!("ES8311 codec initialized at {} Hz", sample_rate_hz);
    Ok(())
}

fn configure_clock<I2C, PinError>(
    i2c: &mut I2C,
    clock: &ClockConfig,
) -> Result<(), Error<I2C::Error, PinError>>
where
    I2C: I2c<SevenBitAddress>,
{
    let mut reg2 = read_reg(i2c, ES8311_CLK_MANAGER_REG02)? & 0x07;
    reg2 |= (clock.pre_div.saturating_sub(1) & 0x07) << 5;
    let pre_multi_bits = match clock.pre_multi {
        1 => 0,
        2 => 1,
        4 => 2,
        8 => 3,
        _ => 0,
    };
    reg2 |= pre_multi_bits << 3;
    write_reg(i2c, ES8311_CLK_MANAGER_REG02, reg2)?;

    let reg5 =
        ((clock.adc_div.saturating_sub(1) & 0x0F) << 4) | (clock.dac_div.saturating_sub(1) & 0x0F);
    write_reg(i2c, ES8311_CLK_MANAGER_REG05, reg5)?;

    let mut reg3 = read_reg(i2c, ES8311_CLK_MANAGER_REG03)? & 0x80;
    reg3 |= (clock.fs_mode & 0x03) << 6;
    reg3 |= clock.adc_osr & 0x3F;
    write_reg(i2c, ES8311_CLK_MANAGER_REG03, reg3)?;

    let mut reg4 = read_reg(i2c, ES8311_CLK_MANAGER_REG04)? & 0x80;
    reg4 |= clock.dac_osr & 0x3F;
    write_reg(i2c, ES8311_CLK_MANAGER_REG04, reg4)?;

    let mut reg7 = read_reg(i2c, ES8311_CLK_MANAGER_REG07)? & 0xC0;
    reg7 |= clock.lrck_h;
    write_reg(i2c, ES8311_CLK_MANAGER_REG07, reg7)?;

    write_reg(i2c, ES8311_CLK_MANAGER_REG08, clock.lrck_l)?;

    let mut reg6 = read_reg(i2c, ES8311_CLK_MANAGER_REG06)? & 0xE0;
    let bclk_val = if clock.bclk_div < 19 {
        clock.bclk_div.saturating_sub(1)
    } else {
        clock.bclk_div
    } & 0x1F;
    reg6 |= bclk_val;
    write_reg(i2c, ES8311_CLK_MANAGER_REG06, reg6)?;

    Ok(())
}

fn write_reg<I2C, PinError>(
    i2c: &mut I2C,
    register: u8,
    value: u8,
) -> Result<(), Error<I2C::Error, PinError>>
where
    I2C: I2c<SevenBitAddress>,
{
    i2c.write(ES8311_I2C_ADDR, &[register, value])
        .map_err(Error::I2c)
}

fn read_reg<I2C, PinError>(i2c: &mut I2C, register: u8) -> Result<u8, Error<I2C::Error, PinError>>
where
    I2C: I2c<SevenBitAddress>,
{
    let mut buf = [0u8; 1];
    i2c.write_read(ES8311_I2C_ADDR, &[register], &mut buf)
        .map_err(Error::I2c)?;
    Ok(buf[0])
}
