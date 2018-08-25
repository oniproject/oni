use generic_array::{ArrayLength, GenericArray};
use generic_array::typenum::{Quot, U8};

use std::ops::Div;

pub struct BitSet<L>
    where L: ArrayLength<u8> + Div<U8>,
          Quot<L, U8>: ArrayLength<u8>
{
    bits: GenericArray<u8, Quot<L, U8>>,
}

#[inline(always)]
fn index(bit: usize) -> usize { bit >> 3 }

#[inline(always)]
fn mask(bit: usize) -> u8 { 1 << (bit & 0b111) }

impl<L> BitSet<L>
    where L: ArrayLength<u8> + Div<U8>,
          Quot<L, U8>: ArrayLength<u8>
{
    #[inline]
    pub fn new() -> Self {
        Self { bits: GenericArray::default() }
    }

    #[inline(always)]
    pub fn num_bits(&self) -> usize { L::to_usize() }

    #[inline]
    pub fn as_slice(&self) -> &[u8] { self.bits.as_slice() }

    #[inline(always)]
    pub unsafe fn get(&self, bit: usize) -> bool {
        *self.bits.get_unchecked(index(bit)) & mask(bit) != 0
    }

    #[inline(always)]
    pub unsafe fn set(&mut self, bit: usize) {
        *self.bits.get_unchecked_mut(index(bit)) |= mask(bit)
    }

    #[inline(always)]
    pub unsafe fn clear(&mut self, bit: usize) {
        *self.bits.get_unchecked_mut(index(bit)) &= !mask(bit)
    }
}

#[test]
fn bitset() {
    unsafe {
        use generic_array::typenum::U32;

        let mut bs = BitSet::<U32>::new();
        assert_eq!(bs.num_bits(), 32);

        bs.set(1);
        bs.set(25);

        assert!(bs.get(1));
        assert!(bs.get(25));

        assert_eq!(bs.as_slice(), &[2, 0, 0, 2]);

        bs.clear(25);
        assert!(!bs.get(25));

        assert_eq!(bs.as_slice(), &[2, 0, 0, 0]);
    }
}
