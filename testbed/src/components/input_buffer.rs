use serde::{
    ser::{Serialize, Serializer},
    de::{self, Visitor, Deserialize, Deserializer},
};
use specs::prelude::*;
use std::fmt;
use oni::sequence::{Sequence, SequenceOps};

#[derive(Component)]
#[storage(DenseVecStorage)]
pub struct InputBuffer {
    pub seq: Sequence<u8>,
    pub ack: [u64; 4],
}

macro wrd64($pos:expr) { ($pos >> 6) as usize }
macro idx64($pos:expr) { ($pos & 63) }

#[derive(Debug, Clone, Copy)]
pub struct Acks<T: fmt::Debug + Copy>(pub T);

struct AcksVisitor;

impl<'de> Visitor<'de> for AcksVisitor {
    type Value = Acks<u128>;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("acks as u128")
    }

    fn visit_u128<E: de::Error>(self, value: u128) -> Result<Self::Value, E> {
        Ok(Acks(value))
    }
}

impl Serialize for Acks<u128> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_u128(self.0)
    }
}
impl<'de> Deserialize<'de> for Acks<u128> {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        deserializer.deserialize_u128(AcksVisitor)
    }
}

impl InputBuffer {
    pub fn new() -> Self {
        Self {
            seq: Sequence::from(0),
            ack: [0; 4],
        }
    }

    #[inline]
    fn clear_between(&mut self, a: u8, b: u8) {
        let (a, b) = (a as u64, b as u64);

        for bit in a..b {
            self.ack[wrd64!(bit)] &= 1 << idx64!(bit);
        }
    }

    #[inline]
    fn can_insert(&self, seq: Sequence<u8>) -> bool {
        seq >= self.seq
    }

    #[inline]
    fn exists(&self, bit: u8) -> bool {
        let bit = bit as u64;
        self.ack[wrd64!(bit)] & (1 << idx64!(bit)) != 0
    }

    pub fn insert(&mut self, seq: Sequence<u8>) -> bool {
        if self.can_insert(seq) {
            if seq.next() > self.seq {
                self.clear_between(self.seq.into(), seq.into());
            }
            self.seq = seq.next();
            true
        } else {
            false
        }
    }

    pub fn generate_ack(&self) -> (Sequence<u8>, Acks<u128>) {
        let ack: u8 = self.seq.prev().into();
        let mut ack_bits = 0;
        for i in 0..128 {
            let seq = ack.wrapping_sub(i);
            if self.exists(seq) {
                ack_bits |= 1 << i;
            }
        }
        (ack.into(), Acks(ack_bits))
    }
}