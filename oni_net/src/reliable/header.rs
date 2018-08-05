use byteorder::{LE, ReadBytesExt};

use crate::seq::Seq;
use crate::utils::UncheckedWriter;
use super::Error;

bitflags! {
    struct Prefix: u8 {
        const ACK_BITS  = 0b0000_1111;
        const SMALL_ACK = 0b1000_0000;
    }
}

#[derive(Debug, Default, PartialEq, Eq, Clone, Copy)]
crate struct Header {
    pub seq: u16,
    pub ack: u16,
    pub ack_bits: u32,
}

impl Header {
    pub const MIN: usize = 4;
    pub const MAX: usize = 9;

    pub fn ack_sequences(self) -> impl Iterator<Item=Seq> {
        let Self { ack, ack_bits, .. } = self;
        (0..32)
            .filter(move |&i| ack_bits & (1 << i) != 0)
            .map(move |i| ack - (i as u16))
            .map(Seq::from)
    }

    pub fn write(self, buf: &mut [u8]) -> usize {
        let Header { seq, ack, ack_bits } = self;
        debug_assert!(buf.len() >= Header::MAX);
        unsafe {
            write_header(buf.as_mut_ptr(), seq, ack, ack_bits)
        }
    }

    pub fn read(mut buf: &[u8]) -> Result<(Self, usize), Error> {
        let prefix = Prefix::from_bits(buf.read_u8()?)
            .ok_or(Error::PacketHeaderInvalid)?;

        let seq = buf.read_u16::<LE>()?;

        let (mut len, ack) = if prefix.contains(Prefix::SMALL_ACK) {
            (4, seq.wrapping_sub(buf.read_u8()? as u16))
        } else {
            (5, buf.read_u16::<LE>()?)
        };

        let mut ack_bits = 0xFFFF_FFFF;
        for i in 0..4 {
            if prefix.contains(Prefix::from_bits(1 << i).unwrap()) {
                ack_bits &= !(0xFF << i * 8);
                ack_bits |= (buf.read_u8()? as u32) << (i * 8);
                len += 1;
            }
        }
        Ok((Header { seq, ack, ack_bits }, len))
    }
}

crate unsafe fn write_header(buf: *mut u8, seq: u16, ack: u16, ack_bits: u32)
    -> usize
{
    let mut buf = UncheckedWriter::new(buf);

    let mut prefix = 0;
    for i in 0..4 {
        let mask = 0xFF << i * 8;
        if ack_bits & mask != mask {
            prefix |= 1 << i;
        }
    }

    let diff = seq.wrapping_sub(ack);

    if diff <= 255 {
        buf.write_u8(prefix | Prefix::SMALL_ACK.bits());
        buf.write_u16(seq);
        buf.write_u8(diff as u8);
    } else {
        buf.write_u8(prefix);
        buf.write_u16(seq);
        buf.write_u16(ack);
    }
    for i in 0..4 {
        let mask = 0xFF << i * 8;
        if ack_bits & mask != mask {
            buf.write_u8(((ack_bits & mask) >> i * 8) as u8);
        }
    }
    let len = buf.diff();
    debug_assert!(len <= Header::MAX);
    len
}

#[test]
fn read_write() {
    println!("size_of reliable::Header: {} u32:{} u64:{} u128:{}",
        std::mem::size_of::<Header>(),
        std::mem::size_of::<(u16, u16, u32)>(),
        std::mem::size_of::<(u16, u16, u64)>(),
        std::mem::size_of::<(u16, u16, u128)>(),
    );

    let tests = [
        // worst case
        // sequence and ack are far apart
        // no packets acked
        (Header { seq: 10_000, ack: 100, ack_bits: 0 }, Header::MAX),
        (Header { seq: 100, ack: 10_000, ack_bits: 0 }, Header::MAX),

        // rare case
        // sequence and ack are far apart
        // significant # of acks are missing
        (Header { seq: 10_000, ack: 100, ack_bits: 0xFEFEFFFE },
            Header::MIN + 1 + 3),
        (Header { seq: 100, ack: 10_000, ack_bits: 0xFEFEFFFE },
            Header::MIN + 1 + 3),

        // ideal case
        // no packet loss
        (Header { seq: 200, ack: 100, ack_bits: 0xFFFFFFFF },
            Header::MIN),
        (Header { seq: 100, ack: 0xFFFF - 100, ack_bits: 0xFFFFFFFF },
            Header::MIN),

        // common case under packet loss
        // sequence and ack are close together
        // some acks are missing
        (Header { seq: 200, ack: 100, ack_bits: 0xFeFe_FeFe }, Header::MIN + 4),
        (Header { seq: 200, ack: 100, ack_bits: 0xFeFe_FeFF }, Header::MIN + 3),
        (Header { seq: 200, ack: 100, ack_bits: 0xFeFe_FFFF }, Header::MIN + 2),
        (Header { seq: 200, ack: 100, ack_bits: 0xFeFF_FFFF }, Header::MIN + 1),
    ];

    let subs = [
        0,
        100,
        200,
        300,
        400,

        0xFFFF - 0,
        0xFFFF - 100,
        0xFFFF - 200,
        0xFFFF - 300,
        0xFFFF - 400,

        0xFFFF / 2,

        0xFFFF / 2 - 100,
        0xFFFF / 2 - 200,
        0xFFFF / 2 - 300,
        0xFFFF / 2 - 400,
        0xFFFF / 2 + 100,
        0xFFFF / 2 + 200,
        0xFFFF / 2 + 300,
        0xFFFF / 2 + 400,
    ];

    for (test, len) in &tests {
        for sub in subs.iter().cloned() {
            for i in 0..32 {
                let write = Header {
                    seq: test.seq.wrapping_add(sub),
                    ack: test.ack.wrapping_add(sub),
                    ack_bits: test.ack_bits.rotate_left(i),
                };

                let mut header = [0u8; Header::MAX];
                let w = write.write(&mut header[..]);
                assert_eq!(w, *len);

                let (read, r) = Header::read(&mut header[..w]).unwrap();
                assert_eq!(r, w);
                assert_eq!(read, write);
            }
        }
    }
}
