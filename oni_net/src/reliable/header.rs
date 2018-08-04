use byteorder::{LE, ReadBytesExt, WriteBytesExt};
use std::io;

#[derive(Copy, Clone, PartialEq, Eq, Debug, Default)]
pub struct Header {
    pub seq: u16,
    pub ack: u16,
    pub ack_bits: u32,
}

impl Header {
    pub const MIN_BYTES: usize = 3;
    pub const MAX_BYTES: usize = 9;

    pub fn write_array(&self) -> ([u8; Self::MAX_BYTES], usize) {
        let mut data: [u8; Self::MAX_BYTES] = unsafe { std::mem::uninitialized() };
        let len = self.write(&mut data[..]).unwrap();
        (data, len)
    }

    pub fn write(&self, mut buffer: &mut [u8]) -> io::Result<usize> {
        let len = buffer.len();

        let mut prefix = 0u8;
        if self.ack_bits & 0x000000FF != 0x000000FF {
            prefix |= 1 << 1;
        }
        if self.ack_bits & 0x0000FF00 != 0x0000FF00 {
            prefix |= 1 << 2;
        }
        if self.ack_bits & 0x00FF0000 != 0x00FF0000 {
            prefix |= 1 << 3;
        }
        if self.ack_bits & 0xFF000000 != 0xFF000000 {
            prefix |= 1 << 4;
        }

        let diff = self.seq.wrapping_sub(self.ack);
        if diff <= 255 {
            buffer.write_u8(prefix | (1 << 5))?;
            buffer.write_u16::<LE>(self.seq)?;
            buffer.write_u8(diff as u8)?;
        } else {
            buffer.write_u8(prefix)?;
            buffer.write_u16::<LE>(self.seq)?;
            buffer.write_u16::<LE>(self.ack)?;
        }

        if self.ack_bits & 0x000000FF != 0x000000FF {
            buffer.write_u8(((self.ack_bits & 0x000000FF) >> 0x00) as u8)?;
        }
        if self.ack_bits & 0x0000FF00 != 0x0000FF00 {
            buffer.write_u8(((self.ack_bits & 0x0000FF00) >> 0x08) as u8)?;
        }
        if self.ack_bits & 0x00FF0000 != 0x00FF0000 {
            buffer.write_u8(((self.ack_bits & 0x00FF0000) >> 0x10) as u8)?;
        }
        if self.ack_bits & 0xFF000000 != 0xFF000000 {
            buffer.write_u8(((self.ack_bits & 0xFF000000) >> 0x18) as u8)?;
        }

        Ok(len - buffer.len())
    }

    pub fn read(&mut self, mut buffer: &[u8]) -> Option<usize> {
        let len = buffer.len();
        if len < Self::MIN_BYTES {
            return None;
        }

        let prefix = buffer.read_u8().ok()?;
        if prefix & 1 != 0 {
            return None;
        }

        let seq = buffer.read_u16::<LE>().ok()?;

        let ack = if prefix & (1<<5) != 0 {
            seq.wrapping_sub(buffer.read_u8().ok()? as u16)
        } else {
            buffer.read_u16::<LE>().ok()?
        };

        /*
        let expected_bytes = 0;
        for i in 1..5 {
            if prefix & (1<<i) {
                expected_bytes += 1;
            }
        }
        */

        let mut ack_bits = 0xFFFFFFFF;
        if prefix & (1<<1) != 0 {
            ack_bits &= 0xFFFFFF00;
            ack_bits |= (buffer.read_u8().ok()? as u32) << 0x00;
        }
        if prefix & (1<<2) != 0 {
            ack_bits &= 0xFFFF00FF;
            ack_bits |= (buffer.read_u8().ok()? as u32) << 0x08;
        }
        if prefix & (1<<3) != 0 {
            ack_bits &= 0xFF00FFFF;
            ack_bits |= (buffer.read_u8().ok()? as u32) << 0x10;
        }
        if prefix & (1<<4) != 0 {
            ack_bits &= 0x00FFFFFF;
            ack_bits |= (buffer.read_u8().ok()? as u32) << 0x18;
        }

        self.seq = seq;
        self.ack = ack;
        self.ack_bits = ack_bits;
        Some(len - buffer.len())
    }
}


#[test]
fn packet_header() {
    let tests = [
        // worst case
        // sequence and ack are far apart
        // no packets acked
        (Header { seq: 10_000, ack: 100, ack_bits: 0 }, Header::MAX_BYTES),
        (Header { seq: 100, ack: 10_000, ack_bits: 0 }, Header::MAX_BYTES),

        // rare case
        // sequence and ack are far apart
        // significant # of acks are missing
        (Header { seq: 10_000, ack: 100, ack_bits: 0xFEFEFFFE }, 1 + 2 + 2 + 3),
        (Header { seq: 100, ack: 10_000, ack_bits: 0xFEFEFFFE }, 1 + 2 + 2 + 3),

        // common case under packet loss
        // sequence and ack are close together
        // some acks are missing
        (Header { seq: 200, ack: 100, ack_bits: 0xFFFEFFFF }, 1 + 2 + 1 + 1),

        // common case under packet loss
        // sequence and ack are close together,
        // some acks are missing
        (Header { seq: 200, ack: 100, ack_bits: 0xFeFe_FeFe }, 1 + 2 + 1 + 4),

        (Header { seq: 200, ack: 100, ack_bits: 0xFeFe_FeFF }, 1 + 2 + 1 + 3),
        (Header { seq: 200, ack: 100, ack_bits: 0xFeFe_FFFe }, 1 + 2 + 1 + 3),
        (Header { seq: 200, ack: 100, ack_bits: 0xFeFF_FeFe }, 1 + 2 + 1 + 3),
        (Header { seq: 200, ack: 100, ack_bits: 0xFFFe_FeFe }, 1 + 2 + 1 + 3),

        (Header { seq: 200, ack: 100, ack_bits: 0xFeFe_FFFF }, 1 + 2 + 1 + 2),
        (Header { seq: 200, ack: 100, ack_bits: 0xFFFF_FeFe }, 1 + 2 + 1 + 2),
        (Header { seq: 200, ack: 100, ack_bits: 0xFFFe_FFFe }, 1 + 2 + 1 + 2),
        (Header { seq: 200, ack: 100, ack_bits: 0xFeFF_FeFF }, 1 + 2 + 1 + 2),

        (Header { seq: 200, ack: 100, ack_bits: 0xFeFF_FFFF }, 1 + 2 + 1 + 1),
        (Header { seq: 200, ack: 100, ack_bits: 0xFFFe_FFFF }, 1 + 2 + 1 + 1),
        (Header { seq: 200, ack: 100, ack_bits: 0xFFFF_FeFF }, 1 + 2 + 1 + 1),
        (Header { seq: 200, ack: 100, ack_bits: 0xFFFF_FFFe }, 1 + 2 + 1 + 1),

        // ideal case
        // no packet loss
        (Header { seq: 200, ack: 100, ack_bits: 0xFFFFFFFF }, 1 + 2 + 1 + 0),

        (Header { seq: 100, ack: 0xFFFF - 100, ack_bits: 0xFFFFFFFF }, 1 + 2 + 1 + 0),
    ];

    let subs = [0, 100, 200, 300, 0xFFFF, 0xFFFF - 100, 0xFFFF - 200, 0xFFFF - 300];
    for (write, len) in &tests {
        for sub in subs.iter().cloned() {
            for i in 0..32 {
                let write = Header {
                    ack_bits: write.ack_bits.rotate_left(i),
                    seq: write.seq.wrapping_sub(sub),
                    ack: write.ack.wrapping_sub(sub)
                };

                let mut header = [0u8; Header::MAX_BYTES];
                let mut read = Header::default();

                let w = write.write(&mut header[..]).unwrap();
                assert_eq!(w, *len);

                let r = read.read(&mut header[..w]).unwrap();
                assert_eq!(r, w);

                assert_eq!(read, write);
            }
        }
    }
}
