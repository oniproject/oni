use byteorder::{LE, ByteOrder};

macro qround($a:expr, $b:expr, $c:expr, $d:expr) {
    $a = $a.wrapping_add($b); $d = ($d ^ $a).rotate_left(16);
    $c = $c.wrapping_add($d); $b = ($b ^ $c).rotate_left(12);
    $a = $a.wrapping_add($b); $d = ($d ^ $a).rotate_left( 8);
    $c = $c.wrapping_add($d); $b = ($b ^ $c).rotate_left( 7);
}

#[inline]
pub fn hchacha20<C: Into<Option<[u8; 16]>>>(dst: &mut [u8; 32], src: &[u8; 16], k: &[u8; 32], c: C) {
    let c = c.into().unwrap_or(*b"expand 32-byte k");

    let mut x0 = LE::read_u32(&c[ 0..]);
    let mut x1 = LE::read_u32(&c[ 4..]);
    let mut x2 = LE::read_u32(&c[ 8..]);
    let mut x3 = LE::read_u32(&c[12..]);

    let mut x4  = LE::read_u32(&k[ 0..]);
    let mut x5  = LE::read_u32(&k[ 4..]);
    let mut x6  = LE::read_u32(&k[ 8..]);
    let mut x7  = LE::read_u32(&k[12..]);

    let mut x8  = LE::read_u32(&k[16..]);
    let mut x9  = LE::read_u32(&k[20..]);
    let mut x10 = LE::read_u32(&k[24..]);
    let mut x11 = LE::read_u32(&k[28..]);

    let mut x12 = LE::read_u32(&src[ 0..]);
    let mut x13 = LE::read_u32(&src[ 4..]);
    let mut x14 = LE::read_u32(&src[ 8..]);
    let mut x15 = LE::read_u32(&src[12..]);

    for _ in 0..10 {
        // odd round
        qround!(x0, x4,  x8, x12); // column 0
        qround!(x1, x5,  x9, x13); // column 1
        qround!(x2, x6, x10, x14); // column 2
        qround!(x3, x7, x11, x15); // column 3
        // even round
        qround!(x0, x5, x10, x15); // diagonal 1 (main diagonal)
        qround!(x1, x6, x11, x12); // diagonal 2
        qround!(x2, x7,  x8, x13); // diagonal 3
        qround!(x3, x4,  x9, x14); // diagonal 4
    }

    LE::write_u32(&mut dst[ 0..],  x0);
    LE::write_u32(&mut dst[ 4..],  x1);
    LE::write_u32(&mut dst[ 8..],  x2);
    LE::write_u32(&mut dst[12..],  x3);
    LE::write_u32(&mut dst[16..], x12);
    LE::write_u32(&mut dst[20..], x13);
    LE::write_u32(&mut dst[24..], x14);
    LE::write_u32(&mut dst[28..], x15);
}
