#![no_std]

pub fn integer_sqrt(value: u32) -> u32 {
    if value <= 1 {
        return value;
    }
    let mut res = 0u32;
    let mut bit = 1u32 << 30;
    let mut value_mut = value;
    while bit > value_mut {
        bit >>= 2;
    }
    while bit != 0 {
        if value_mut >= res + bit {
            value_mut -= res + bit;
            res = (res >> 1) + bit;
        } else {
            res >>= 1;
        }
        bit >>= 2;
    }
    res
}
