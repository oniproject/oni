use std::cmp::Ordering;

#[derive(Debug, Default, Hash, Eq, PartialEq, Clone, Copy)]
pub struct Sequence(u16);

impl From<u16> for Sequence {
    fn from(v: u16) -> Self {
        Sequence(v)
    }
}

impl Sequence {
    pub fn next(self) -> Self {
        Sequence(self.0.wrapping_add(1))
    }

    pub fn prev(self) -> Self {
        Sequence(self.0.wrapping_sub(1))
    }

    fn _more_recent(&self, other: Self) -> bool {
        let half = u16::max_value() / 2;
        let (a, b) = (self.0, other.0);
        a > b && a - b <= half ||
        b > a && b - a > half
    }
}


impl PartialOrd for Sequence {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Sequence {
    fn cmp(&self, other: &Self) -> Ordering {
        if self.0 == other.0 {
            Ordering::Equal
        } else {
            let half = u16::max_value() / 2;
            if self.0.wrapping_sub(other.0) < half {
                Ordering::Greater
            } else {
                Ordering::Less
            }
        }
    }
}



#[test]
fn sequence() {
    let a = Sequence(0);
    let b = Sequence(0xFFFF);

    assert_eq!(a.prev(), b);
    assert_eq!(b.next(), a);

    let tests = &[
        (1u16, 0u16, 0xFFFFu16),
        (2u16, 1u16, 0u16),
        (3u16, 2u16, 1u16),

        (0xFFFFu16, 0xFFFFu16 - 1, 0xFFFFu16 - 2),
        (0u16, 0xFFFFu16, 0xFFFFu16 - 1),
    ];

    for (a, b, c) in tests.into_iter().cloned() {
        let a = Sequence(a);
        let b = Sequence(b);
        let c = Sequence(c);

        assert!(a > b, "a > b: {:?} {:?}", a, b);
        assert!(b > c, "b > c: {:?} {:?}", b, c);
        assert!(a > c, "a > c: {:?} {:?}", a, c);
        assert!(b < a, "b < a: {:?} {:?}", b, a);
        assert!(c < b, "c < b: {:?} {:?}", c, b);
        assert!(c < a, "c < a: {:?} {:?}", c, a);
    }
}

/*
#[inline]
fn sequence_more_recent<U: Unsigned>(first: U, second: U) -> bool {
    let max = U::max_value();
    let two = U::one() + U::one();
    first > second && first-second <= max/two || second > first && second-first > max/two
}

#[derive(Debug, Hash, Eq, PartialEq, Clone, Copy)]
pub struct Sequence<U: Unsigned>(pub U);

impl<U: Unsigned> Sequence<U> {

}
*/

pub mod packet_sequence {
    ///! algorithm: encode a mask of 7 bits into one byte.
    ///! Each bit is set if byte n in the sequence is non-zero.
    ///! Non-zero bytes follow after the first byte.
    ///! The low byte in sequence is always sent (hence 7 bits, not 8).

    pub fn compress(sequence: u64, bytes: &mut [u8; 8]) -> (u8, &[u8]) {
        let mut prefix = 0;
        let mut count = 0;
        let mut mask = 0xFF00000000000000u64;
        for i in (1..8).rev() {
            let current = ((sequence&mask) >> (i*8)) as u8;
            if current != 0 {
                bytes[count] = current;
                count += 1;
                prefix |= 1 << (i-1);
            }
            mask >>= 8;
        }
        assert_eq!(prefix & (1<<7), 0);
        bytes[count] = (sequence & 0xFF) as u8;
        (prefix, &bytes[..count + 1])
    }

    pub fn bytes(prefix_byte: u8) -> usize {
        let mut count = 0;
        for i in (1..8).rev() {
            if prefix_byte & (1 << (i-1)) != 0 {
                count += 1;
            }
        }
        count + 1
    }

    pub fn decompress(prefix_byte: u8, sequence_bytes: &[u8; 8]) -> u64 {
        let mut sequence = 0;
        let mut index = 0;
        for i in (1..8).rev() {
            if prefix_byte & (1 << (i-1)) != 0 {
                sequence |= (sequence_bytes[index] as u64) << (i*8);
                index += 1;
            }
        }
        sequence | sequence_bytes[index] as u64
    }
}

#[test]
fn packet_sequence() {
    const SEQUENCE: u64 = 0x00001100223344;

    let mut bytes = [0u8; 8];
    let (prefix, len) = {
        let (prefix_byte, bytes) = packet_sequence::compress(SEQUENCE, &mut bytes);

        assert_eq!(prefix_byte & (1<<7), 0);
        assert_eq!(prefix_byte, 1 | (1<<1) | (1<<3));
        assert_eq!(bytes.len(), 4);
        assert_eq!(bytes[0], 0x11);
        assert_eq!(bytes[1], 0x22);
        assert_eq!(bytes[2], 0x33);
        assert_eq!(bytes[3], 0x44);
        (prefix_byte, bytes.len())
    };

    let decoded = packet_sequence::bytes(prefix);
    assert_eq!(decoded, len);

    let sequence = packet_sequence::decompress(prefix, &mut bytes);
    assert_eq!(sequence, SEQUENCE);
}
