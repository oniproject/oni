use byteorder::{LE, ByteOrder};

// TODO: big endian for reading

pub trait WritePrefixVarint: std::io::Write {
    fn write_prefix_varint(&mut self, seq: u64) -> std::io::Result<()> {
        self.write_prefix_varint_custom(seq, 0)
    }
    fn write_prefix_varint_custom(&mut self, seq: u64, min: u32) -> std::io::Result<()> {
        let mut buf = unsafe { std::mem::uninitialized() };
        let buf = write_varint(&mut buf, seq, min);
        self.write_all(buf)
    }
}

impl<T: std::io::Write> WritePrefixVarint for T {}

#[inline(always)]
pub fn read_z(b: u8) -> u32 {
    b.trailing_zeros() + 1
}

/// `z >= 1 && z <= 9`
#[inline(always)]
pub unsafe fn read_varint64_unchecked(p: *const u8, z: u32) -> u64 {
    #![allow(clippy::cast_ptr_alignment)]
    assert!(cfg!(target_endian = "little"), "big endian doesn't support");
    if z == 9 {
        (p.add(1) as *const u64).read_unaligned()
    } else {
        read_varint56_unchecked(p, z)
    }
}

/// `z >= 1 && z <= 8`
#[inline(always)]
pub unsafe fn read_varint56_unchecked(p: *const u8, z: u32) -> u64 {
    #![allow(clippy::cast_ptr_alignment)]
    assert!(cfg!(target_endian = "little"), "big endian doesn't support");
    let u = 64 - 8 * z;
    ((p as *const u64).read_unaligned() << u) >> (u + z)
}

#[inline(always)]
pub fn read_varint(buf: &[u8]) -> Result<u64, ()> {
    if buf.is_empty() { return Err(()); }
    let z = read_z(buf[0]);
    if buf.len() < z as usize { return Err(()); }
    unsafe {
        Ok(read_varint64_unchecked(buf.as_ptr(), z))
    }
}

#[inline(always)]
pub fn write_varint(buf: &mut [u8; 9], seq: u64, min: u32) -> &[u8] {
    assert!(min <= 8);
    let bits = (64 - (seq | 1).leading_zeros()).max(min * 7);
    let bytes = 1 + (bits - 1) / 7;

    if bits > 56 {
        buf[0] = 0u8;
        LE::write_u64(&mut buf[1..], seq);
        &buf[..]
    } else {
        let mut x = (2 * seq + 1) << (bytes - 1);
        for i in 0..bytes {
            buf[i as usize] = (x & 0xff) as u8;
            x >>= 8;
        }
        &buf[..bytes as usize]
    }
}

#[test]
fn safe() {
    let tests: &[(u64, usize)] = &[
        (0x0000_0000_0000_0000, 1),
        (0x0000_0000_0000_0001, 1),
        (0x0000_0000_0000_007F, 1),
        (0x0000_0000_0000_0080, 2),
        (0x0000_0000_0000_3FFF, 2),
        (0x0000_0000_0000_4000, 3),
        (0x0000_0000_001F_FFFF, 3),
        (0x0000_0000_0020_0000, 4),
        (0x0000_0000_0FFF_FFFF, 4),
        (0x0000_0000_1000_0000, 5),
        (0x0000_0007_FFFF_FFFF, 5),
        (0x0000_0008_0000_0000, 6),
        (0x0000_03FF_FFFF_FFFF, 6),
        (0x0000_0400_0000_0000, 7),
        (0x0001_FFFF_FFFF_FFFF, 7),
        (0x0002_0000_0000_0000, 8),
        (0x00FF_FFFF_FFFF_FFFF, 8),
        (0x0100_0000_0000_0000, 9),
        (0xFFFF_FFFF_FFFF_FFFe, 9),

        (0xFFFF_FFFF_FFFF_FFFF, 9),
    ];

    for (value, expected_len) in tests.iter().cloned() {
        let mut out = [0u8; 9];
        let out = write_varint(&mut out, value, 0);
        assert_eq!(out.len(), expected_len);
        assert_eq!(read_varint(out), Ok(value));
    }
}

#[test]
fn safe2() {
    let tests: &[(u64, usize)] = &[
        (0x0000_0000_0000_0000, 2), // XXX
        (0x0000_0000_0000_0001, 2),
        (0x0000_0000_0000_007F, 2),
        (0x0000_0000_0000_0080, 2),
        (0x0000_0000_0000_3FFF, 2),
        (0x0000_0000_0000_4000, 3),
        (0x0000_0000_001F_FFFF, 3),
        (0x0000_0000_0020_0000, 4),
        (0x0000_0000_0FFF_FFFF, 4),
        (0x0000_0000_1000_0000, 5),
        (0x0000_0007_FFFF_FFFF, 5),
        (0x0000_0008_0000_0000, 6),
        (0x0000_03FF_FFFF_FFFF, 6),
        (0x0000_0400_0000_0000, 7),
        (0x0001_FFFF_FFFF_FFFF, 7),
        (0x0002_0000_0000_0000, 8),
        (0x00FF_FFFF_FFFF_FFFF, 8),
        (0x0100_0000_0000_0000, 9),
        (0xFFFF_FFFF_FFFF_FFFe, 9),

        (0xFFFF_FFFF_FFFF_FFFF, 9),
    ];

    for (value, expected_len) in tests.iter().cloned() {
        let mut out = [0u8; 9];
        let out = write_varint(&mut out, value, 2);
        assert_eq!(out.len(), expected_len);
        assert_eq!(read_varint(out), Ok(value));
    }
}

#[test]
fn varint64() {
    let tests: &[(u64, usize)] = &[
        (0x0000_0000_0000_0000, 1),
        (0x0000_0000_0000_0001, 1),
        (0x0000_0000_0000_007F, 1),
        (0x0000_0000_0000_0080, 2),
        (0x0000_0000_0000_3FFF, 2),
        (0x0000_0000_0000_4000, 3),
        (0x0000_0000_001F_FFFF, 3),
        (0x0000_0000_0020_0000, 4),
        (0x0000_0000_0FFF_FFFF, 4),
        (0x0000_0000_1000_0000, 5),
        (0x0000_0007_FFFF_FFFF, 5),
        (0x0000_0008_0000_0000, 6),
        (0x0000_03FF_FFFF_FFFF, 6),
        (0x0000_0400_0000_0000, 7),
        (0x0001_FFFF_FFFF_FFFF, 7),
        (0x0002_0000_0000_0000, 8),
        (0x00FF_FFFF_FFFF_FFFF, 8),
        (0x0100_0000_0000_0000, 9),
        (0xFFFF_FFFF_FFFF_FFFe, 9),

        (0xFFFF_FFFF_FFFF_FFFF, 9),
    ];

    for (value, expected_len) in tests.iter().cloned() {
        let mut out = [0u8; 9];
        let out = write_varint(&mut out, value, 0);
        let z = read_z(out[0]);
        assert_eq!(z, expected_len as u32);
        println!("z: {}", z);
        unsafe {
            let s = read_varint64_unchecked(out.as_ptr(), z);
            assert_eq!(s, value);
        }
    }
}

#[test]
fn prefix_varint56() {
    let tests: &[(u64, usize)] = &[
        (0x0000_0000_0000_0000, 1),
        (0x0000_0000_0000_0001, 1),
        (0x0000_0000_0000_007F, 1),
        (0x0000_0000_0000_0080, 2),
        (0x0000_0000_0000_3FFF, 2),
        (0x0000_0000_0000_4000, 3),
        (0x0000_0000_001F_FFFF, 3),
        (0x0000_0000_0020_0000, 4),
        (0x0000_0000_0FFF_FFFF, 4),
        (0x0000_0000_1000_0000, 5),
        (0x0000_0007_FFFF_FFFF, 5),
        (0x0000_0008_0000_0000, 6),
        (0x0000_03FF_FFFF_FFFF, 6),
        (0x0000_0400_0000_0000, 7),
        (0x0001_FFFF_FFFF_FFFF, 7),
        (0x0002_0000_0000_0000, 8),
        (0x00FF_FFFF_FFFF_FFFF, 8),
    ];

    for (value, expected_len) in tests.iter().cloned() {
        let mut out = [0u8; 9];
        let out = write_varint(&mut out, value, 0);
        let z = read_z(out[0]);
        assert_eq!(z, expected_len as u32);
        println!("z: {}", z);
        unsafe {
            let s = read_varint56_unchecked(out.as_ptr(), z);
            assert_eq!(s, value);
        }
    }
}
