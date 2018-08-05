use byteorder::{LE, WriteBytesExt, ReadBytesExt};
use std::io::{self, Read, Write};

// prefix 8 (
//     Regular 4-8 (
//         sequence 16
//         ack 8|16
//         ack_bits 8|16|24|32
//     )
//     Fragment 4 (
//         sequence 16
//         fragment_id 8
//         num_fragments 8
//
//         +Regular if fragment_id == 0
//     )
// )
bitflags! {
    struct Prefix: u8 {
        const REGULAR    = 0;
        const FRAGMENTED = 1;

        const ACK_BITS_0 = 1 << 1;
        const ACK_BITS_1 = 1 << 2;
        const ACK_BITS_2 = 1 << 3;
        const ACK_BITS_3 = 1 << 4;

        const LARGE_ACK  = 1 << 5;
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum Header {
    Regular(Regular),
    Fragment(Fragment),
}

impl Header {
    pub fn read(mut buf: &[u8]) -> io::Result<(Self, usize)> {
        let frag = Prefix::from_bits(buf[0])
            .unwrap()
            .contains(Prefix::FRAGMENTED);
        if frag {
            let (p, len) = Fragment::read(buf)?;
            Ok((Header::Fragment(p), len))
        } else {
            let (p, len) = Regular::read(buf)?;
            Ok((Header::Regular(p), len))
        }
    }
}

#[derive(Debug, Default, PartialEq, Eq, Clone, Copy)]
pub struct Regular {
    pub seq: u16,
    pub ack: u16,
    pub ack_bits: u32,
}

impl Regular {
    pub const MIN_BYTES: usize = 3;
    pub const MAX_BYTES: usize = 9;

    pub fn new(seq: u16, ack: u16, ack_bits: u32) -> Self {
        Self { seq, ack, ack_bits }
    }

    pub fn write(self, mut buf: &mut [u8]) -> io::Result<usize> {
        let Regular { seq, ack, ack_bits } = self;

        let mut prefix = Prefix::REGULAR;
        if ack_bits & 0x000000FF != 0x000000FF {
            prefix |= Prefix::ACK_BITS_0;
        }
        if ack_bits & 0x0000FF00 != 0x0000FF00 {
            prefix |= Prefix::ACK_BITS_1;
        }
        if ack_bits & 0x00FF0000 != 0x00FF0000 {
            prefix |= Prefix::ACK_BITS_2;
        }
        if ack_bits & 0xFF000000 != 0xFF000000 {
            prefix |= Prefix::ACK_BITS_3;
        }

        let diff = seq.wrapping_sub(self.ack);
        let mut len = if diff <= 255 {
            buf.write_u8((prefix | Prefix::LARGE_ACK).bits())?;
            buf.write_u16::<LE>(seq)?;
            buf.write_u8(diff as u8)?;
            4
        } else {
            buf.write_u8(prefix.bits())?;
            buf.write_u16::<LE>(seq)?;
            buf.write_u16::<LE>(ack)?;
            5
        };

        if ack_bits & 0x0000_00FF != 0x0000_00FF {
            buf.write_u8((ack_bits & 0x_0000_00FF) as u8)?;
            len += 1;
        }
        if ack_bits & 0x0000_FF00 != 0x0000_FF00 {
            buf.write_u8(((ack_bits & 0x_0000_FF00) >> 8) as u8)?;
            len += 1;
        }
        if ack_bits & 0x00FF_0000 != 0x00FF_0000 {
            buf.write_u8(((ack_bits & 0x_00FF_0000) >> 16) as u8)?;
            len += 1;
        }
        if ack_bits & 0xFF00_0000 != 0xFF00_0000 {
            buf.write_u8(((ack_bits & 0x_FF00_0000) >> 24) as u8)?;
            len += 1;
        }
        Ok(len)
    }

    pub fn read(mut buf: &[u8]) -> io::Result<(Self, usize)> {
        let prefix = buf.read_u8()?;
        assert!(prefix & 1 == 0, "invalid regular header prefix byte");
        let prefix = Prefix::from_bits(prefix).unwrap();

        let seq = buf.read_u16::<LE>()?;

        let (mut len, ack) = if prefix.contains(Prefix::LARGE_ACK) {
            (4, seq.wrapping_sub(buf.read_u8()? as u16))
        } else {
            (5, buf.read_u16::<LE>()?)
        };

        let mut ack_bits = 0xFFFF_FFFF;
        if prefix.contains(Prefix::ACK_BITS_0) {
            ack_bits &= 0xFFFF_FF00;
            ack_bits |= buf.read_u8()? as u32;
            len += 1;
        }
        if prefix.contains(Prefix::ACK_BITS_1) {
            ack_bits &= 0xFFFF_00FF;
            ack_bits |= (buf.read_u8()? as u32) << 8;
            len += 1;
        }
        if prefix.contains(Prefix::ACK_BITS_2) {
            ack_bits &= 0xFF00_FFFF;
            ack_bits |= (buf.read_u8()? as u32) << 16;
            len += 1;
        }
        if prefix.contains(Prefix::ACK_BITS_3) {
            ack_bits &= 0x00FF_FFFF;
            ack_bits |= (buf.read_u8()? as u32) << 24;
            len += 1;
        }

        Ok((Regular { seq, ack, ack_bits }, len))
    }
}

#[derive(Debug, Default, PartialEq, Eq, Clone, Copy)]
pub struct Fragment {
    pub seq: u16,
    pub id: u8,
    pub total: u8,
}

impl Fragment {
    pub const BYTES: usize = 5;

    pub fn new(seq: u16, id: u8, total: u8) -> Self {
        Self { seq, id, total }
    }

    pub fn validate(&self, max_fragments: u8, size: usize, bytes: usize) -> bool {
        !(
            self.total > max_fragments ||
            self.id >= max_fragments ||
            bytes > size ||
            (self.id != self.total - 1 && bytes != size)
        )
    }

    pub fn write(self, mut buf: &mut [u8]) -> io::Result<usize> {
        let Fragment { seq, id, total } = self;
        buf.write_u8(0x01)?; // prefix
        buf.write_u16::<LE>(seq)?;
        buf.write_u8(id)?;
        buf.write_u8(total)?;
        Ok(Self::BYTES)
    }

    pub fn read(mut buf: &[u8]) -> io::Result<(Self, usize)> {
        let prefix = buf.read_u8()?;
        assert!(prefix & 1 == 1, "invalid fragment header prefix byte");
        Ok((Fragment {
            seq: buf.read_u16::<LE>()?,
            id: buf.read_u8()?,
            total: buf.read_u8()?,
        }, Self::BYTES))
    }
}

#[test]
fn regular_header() {
    let tests = [
        // worst case
        // sequence and ack are far apart
        // no packets acked
        (Regular { seq: 10_000, ack: 100, ack_bits: 0 }, Regular::MAX_BYTES),
        (Regular { seq: 100, ack: 10_000, ack_bits: 0 }, Regular::MAX_BYTES),

        // rare case
        // sequence and ack are far apart
        // significant # of acks are missing
        (Regular { seq: 10_000, ack: 100, ack_bits: 0xFEFEFFFE }, 1 + 2 + 2 + 3),
        (Regular { seq: 100, ack: 10_000, ack_bits: 0xFEFEFFFE }, 1 + 2 + 2 + 3),

        // common case under packet loss
        // sequence and ack are close together
        // some acks are missing
        (Regular { seq: 200, ack: 100, ack_bits: 0xFFFEFFFF }, 1 + 2 + 1 + 1),

        // common case under packet loss
        // sequence and ack are close together,
        // some acks are missing
        (Regular { seq: 200, ack: 100, ack_bits: 0xFeFe_FeFe }, 1 + 2 + 1 + 4),

        (Regular { seq: 200, ack: 100, ack_bits: 0xFeFe_FeFF }, 1 + 2 + 1 + 3),
        (Regular { seq: 200, ack: 100, ack_bits: 0xFeFe_FFFe }, 1 + 2 + 1 + 3),
        (Regular { seq: 200, ack: 100, ack_bits: 0xFeFF_FeFe }, 1 + 2 + 1 + 3),
        (Regular { seq: 200, ack: 100, ack_bits: 0xFFFe_FeFe }, 1 + 2 + 1 + 3),

        (Regular { seq: 200, ack: 100, ack_bits: 0xFeFe_FFFF }, 1 + 2 + 1 + 2),
        (Regular { seq: 200, ack: 100, ack_bits: 0xFFFF_FeFe }, 1 + 2 + 1 + 2),
        (Regular { seq: 200, ack: 100, ack_bits: 0xFFFe_FFFe }, 1 + 2 + 1 + 2),
        (Regular { seq: 200, ack: 100, ack_bits: 0xFeFF_FeFF }, 1 + 2 + 1 + 2),

        (Regular { seq: 200, ack: 100, ack_bits: 0xFeFF_FFFF }, 1 + 2 + 1 + 1),
        (Regular { seq: 200, ack: 100, ack_bits: 0xFFFe_FFFF }, 1 + 2 + 1 + 1),
        (Regular { seq: 200, ack: 100, ack_bits: 0xFFFF_FeFF }, 1 + 2 + 1 + 1),
        (Regular { seq: 200, ack: 100, ack_bits: 0xFFFF_FFFe }, 1 + 2 + 1 + 1),

        // ideal case
        // no packet loss
        (Regular { seq: 200, ack: 100, ack_bits: 0xFFFFFFFF }, 1 + 2 + 1 + 0),

        (Regular { seq: 100, ack: 0xFFFF - 100, ack_bits: 0xFFFFFFFF }, 1 + 2 + 1 + 0),
    ];

    let subs = [0, 100, 200, 300, 0xFFFF, 0xFFFF - 100, 0xFFFF - 200, 0xFFFF - 300];
    for (test, len) in &tests {
        for sub in subs.iter().cloned() {
            for i in 0..32 {
                let write = Regular {
                    seq: test.seq.wrapping_sub(sub),
                    ack: test.ack.wrapping_sub(sub),
                    ack_bits: test.ack_bits.rotate_left(i),
                };

                let mut header = [0u8; Regular::MAX_BYTES];
                let w = write.write(&mut header[..]).unwrap();
                assert_eq!(w, *len);

                let (read, r) = Regular::read(&mut header[..w]).unwrap();
                assert_eq!(r, w);
                assert_eq!(read, write);
            }
        }
    }
}
