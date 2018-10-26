use serde::{
    ser::{Serialize, Serializer},
    de::{Deserialize, Deserializer},
};
use specs::prelude::*;
use std::fmt;
use oni_reliable::{Sequence, SequenceOps};

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

impl Serialize for Acks<u128> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_u128(self.0)
    }
}
impl<'de> Deserialize<'de> for Acks<u128> {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        Ok(Acks(u128::deserialize(deserializer)?))
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
        let (a, b) = (u64::from(a), u64::from(b));

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
        let bit = u64::from(bit);
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
