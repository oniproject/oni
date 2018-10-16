use generic_array::{ArrayLength, GenericArray};
use generic_array::typenum::{Quot, Sum, U7, U8, U16, U32, U64, U128, U256, U512, U1024};
use std::mem::transmute;
use std::ops::{Div, Add};

pub type BitSet8 = BitSet<U8>;
pub type BitSet16 = BitSet<U16>;
pub type BitSet32 = BitSet<U32>;
pub type BitSet128 = BitSet<U128>;
pub type BitSet256 = BitSet<U256>;
pub type BitSet512 = BitSet<U512>;
pub type BitSet1024 = BitSet<U1024>;

#[derive(Default, Clone)]
pub struct BitSet<L>
    where L: ArrayLength<u8> + Div<U8> + Add<U7>,
          <L as Add<U7>>::Output: Div<U8>,
          Quot<Sum<L, U7>, U8>: ArrayLength<u8>,
{
    bits: GenericArray<u8, Quot<Sum<L, U7>, U8>>,
}

#[inline(always)]
fn index(bit: usize) -> usize { bit >> 3 }

#[inline(always)]
fn mask(bit: usize) -> u8 { 1 << (bit & 0b111) }

impl<L> BitSet<L>
    where L: ArrayLength<u8> + Div<U8> + Add<U7>,
          <L as Add<U7>>::Output: Div<U8>,
          Quot<Sum<L, U7>, U8>: ArrayLength<u8>,
{
    #[inline]
    pub fn new() -> Self {
        Self { bits: GenericArray::default() }
    }

    #[inline(always)]
    pub fn len(&self) -> usize { L::to_usize() }

    #[inline]
    pub fn as_slice(&self) -> &[u8] { self.bits.as_slice() }

    #[inline(always)]
    pub fn get(&self, bit: usize) -> bool {
        assert!(bit < L::to_usize());
        unsafe { self.get_unchecked(bit) }
    }

    #[inline(always)]
    pub fn set(&mut self, bit: usize) {
        assert!(bit < L::to_usize());
        unsafe { self.set_unchecked(bit) }
    }

    #[inline(always)]
    pub fn clear(&mut self, bit: usize) {
        assert!(bit < L::to_usize());
        unsafe { self.clear_unchecked(bit) }
    }

    #[inline(always)]
    pub unsafe fn get_unchecked(&self, bit: usize) -> bool {
        *self.bits.get_unchecked(index(bit)) & mask(bit) != 0
    }

    #[inline(always)]
    pub unsafe fn set_unchecked(&mut self, bit: usize) {
        *self.bits.get_unchecked_mut(index(bit)) |= mask(bit)
    }

    #[inline(always)]
    pub unsafe fn clear_unchecked(&mut self, bit: usize) {
        *self.bits.get_unchecked_mut(index(bit)) &= !mask(bit)
    }
}

impl BitSet<U8> {
    #[inline(always)]
    pub fn to_array(&self) -> [u8; 1] { unsafe { transmute(self.bits) } }
    #[inline(always)]
    pub fn to_u8(&self) -> u8 { unsafe { transmute(self.bits) } }
}

impl BitSet<U16> {
    #[inline(always)]
    pub fn to_array(&self) -> [u8; 2] { unsafe { transmute(self.bits) } }
    #[inline(always)]
    pub fn to_u16(&self) -> u16 { u16::from_be_bytes(self.to_array()) }
}

impl BitSet<U32> {
    #[inline(always)]
    pub fn to_array(&self) -> [u8; 4] { unsafe { transmute(self.bits) } }
    #[inline(always)]
    pub fn to_u32(&self) -> u32 { u32::from_be_bytes(self.to_array()) }
}

impl BitSet<U64> {
    #[inline(always)]
    pub fn to_array(&self) -> [u8; 8] { unsafe { transmute(self.bits) } }
    #[inline(always)]
    pub fn to_u64(&self) -> u64 { u64::from_be_bytes(self.to_array()) }
}

impl BitSet<U128> {
    #[inline(always)]
    pub fn to_array(&self) -> [u8; 16] { unsafe { transmute(self.bits) } }
    #[inline(always)]
    pub fn to_u128(&self) -> u128 { u128::from_be_bytes(self.to_array()) }
}

impl BitSet<U256> {
    #[inline(always)]
    pub fn to_array(&self) -> [u8; 32] { unsafe { transmute(self.bits) } }
}

impl BitSet<U512> {
    #[inline(always)]
    pub fn to_array(&self) -> [u8; 64] { unsafe { transmute(self.bits) } }
}

impl BitSet<U1024> {
    #[inline(always)]
    pub fn to_array(&self) -> [u8; 128] { unsafe { transmute(self.bits) } }
}

#[test]
fn bitset() {
    let mut bs = BitSet32::new();
    assert_eq!(bs.len(), 32);

    bs.set(1);
    bs.set(25);

    assert!(bs.get(1));
    assert!(bs.get(25));

    assert_eq!(bs.as_slice(), &[2, 0, 0, 2]);
    assert_eq!(bs.to_array(), [2, 0, 0, 2]);

    bs.clear(25);
    assert!(!bs.get(25));

    assert_eq!(bs.as_slice(), &[2, 0, 0, 0]);

    use generic_array::typenum::{U24, U25, U31, U33};

    let bs = BitSet::<U24>::new();
    assert_eq!(bs.len(), 24);
    assert_eq!(bs.bits.len(), 3);

    let bs = BitSet::<U25>::new();
    assert_eq!(bs.len(), 25);
    assert_eq!(bs.bits.len(), 4);

    let bs = BitSet::<U31>::new();
    assert_eq!(bs.len(), 31);
    assert_eq!(bs.bits.len(), 4);

    let bs = BitSet::<U33>::new();
    assert_eq!(bs.len(), 33);
    assert_eq!(bs.bits.len(), 5);
}
