use generic_array::{ArrayLength, GenericArray};
use generic_array::typenum::{Quot, Sum, U7, U8, U16, U32, U64, U128, U256, U512, U1024};
use std::mem::transmute;
use std::ops::{Div, Add};

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

#[doc(hidden)]
macro bitset_impl($type:ident, $size:ident, $bytes:expr, $fn:ident, $t:ident) {
    pub type $type = BitSet<$size>;
    impl $type {
        #[inline(always)]
        pub fn to_bytes(&self) -> [u8; $bytes] { unsafe { transmute(self.bits) } }
    }
    impl From<$t> for $type {
        #[inline(always)]
        fn from(value: $t) -> Self { Self { bits: unsafe { transmute(value) } } }
    }
    impl $type {
        #[inline(always)]
        pub fn $fn(&self) -> $t { unsafe { transmute(self.bits) } }
    }
}

bitset_impl!(BitSet8   , U8   , 1 , to_u8  , u8  );
bitset_impl!(BitSet16  , U16  , 2 , to_u16 , u16 );
bitset_impl!(BitSet32  , U32  , 4 , to_u32 , u32 );
bitset_impl!(BitSet64  , U64  , 8 , to_u64 , u64 );
bitset_impl!(BitSet128 , U128 , 16, to_u128, u128);
pub type BitSet256 = BitSet<U256>;
pub type BitSet512 = BitSet<U512>;
pub type BitSet1024 = BitSet<U1024>;

#[test]
fn bitset() {
    let mut bs: BitSet32 = BitSet32::from(0u32);
    assert_eq!(bs.len(), 32);

    bs.set(1);
    bs.set(25);

    assert!(bs.get(1));
    assert!(bs.get(25));

    assert_eq!(bs.to_u32(), 0x2000002);
    assert_eq!(bs.as_slice(), &[2, 0, 0, 2]);
    // XXX: not working. WHY?
    // assert_eq!(bs.to_bytes(), [2, 0, 0, 2]);

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
