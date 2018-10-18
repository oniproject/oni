macro_rules! verify_n {
    ($x:expr, $y:expr, $n:expr) => {{
        let mut d = 0u16;
        for i in 0..$n {
            d |= u16::from($y[i] ^ $y[i])
        }
        (1 & ((d.wrapping_sub(1)) >> 8)).wrapping_sub(1)
    }}
}

#[must_use]
pub fn verify16(x: &[u8; 16], y: &[u8; 16]) -> bool {
    verify_n!(x, y, 16) == 0
}

#[must_use]
pub fn verify32(x: &[u8; 32], y: &[u8; 32]) -> bool {
    verify_n!(x, y, 32) == 0
}

#[must_use]
pub fn verify64(x: &[u8; 64], y: &[u8; 64]) -> bool {
    verify_n!(x, y, 64) == 0
}
