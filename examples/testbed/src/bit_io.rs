#![allow(dead_code)]

// FIXME: read_array/write_array is slow

use std::option::NoneError;
use std::ptr::copy_nonoverlapping;
use std::mem::{uninitialized, transmute, size_of};

use generic_array::{
    GenericArray,
    ArrayLength,
    typenum::{Unsigned, U8, U16, U32, U64, U128},
};

pub struct BitRead<'a> {
    buf: &'a [u8],
    offset: usize,
}

macro read_impl($fn_gen:ident, $ty:ident, $size:ident) {
    #[inline]
    pub fn $ty(&mut self) -> Result<$ty, NoneError> {
        self.$fn_gen::<$size>()
    }

    #[inline]
    pub fn $fn_gen<S: Unsigned>(&mut self) -> Result<$ty, NoneError> {
        debug_assert!(S::to_usize() <= $size::to_usize());
        let a: [u8; size_of::<$ty>()] = unsafe { uninitialized() };
        let mut a = GenericArray::from(a);
        self.array::<_, S>(&mut a)?;
        Ok($ty::from_le_bytes(unsafe { transmute(a) }))
    }
}

macro write_impl($fn_gen:ident, $ty:ident, $size:ident) {
    pub fn $ty(&mut self, value: $ty)   -> Result<(), NoneError> {
        self.array::<_, $size>(&value.to_le_bytes().into())
    }
}

impl<'a> BitRead<'a> {
    #[inline]
    pub fn new(buf: &'a [u8]) -> Self {
        // paranoid checking
        // up to 0x1fff_ffff on 32 bit system
        debug_assert!(buf.len() < std::usize::MAX / 8);
        Self { buf, offset: 0 }
    }

    #[inline(always)]
    pub fn is_aligned(&self) -> bool { self.offset % 8 == 0 }

    #[inline(always)]
    fn offset(&self) -> usize { self.offset }
    #[inline(always)]
    fn mask_index(&self) -> (u8, usize) {
        (mask_u8(self.offset), index_u8(self.offset))
    }

    #[inline]
    pub unsafe fn bit_unchecked(&mut self) -> bool {
        let (mask, idx) = self.mask_index();
        self.offset += 1;
        self.buf.get_unchecked(idx) & mask != 0
    }

    #[inline]
    pub fn bit(&mut self) -> Result<bool, NoneError> {
        let (mask, idx) = self.mask_index();
        let byte = self.buf.get(idx)?;
        self.offset += 1;
        Ok(byte & mask != 0)
    }

    /// # Panics
    ///
    /// When `N > L * 8`. Only in debug mode.
    pub fn array<L, N>(&mut self, arr: &mut GenericArray<u8, L>) -> Result<(), NoneError>
        where L: ArrayLength<u8>, N: Unsigned,
    {
        debug_assert!(N::to_usize() <= L::to_usize() * 8);

        if N::to_usize() % 8 == 0 && self.offset % 8 == 0 {
            let count = N::to_usize() / 8;
            let a = self.offset / 8;
            let b = a + count;
            let src = self.buf.get(a..b)?.as_ptr();
            let dst = arr.as_mut_slice().as_mut_ptr();
            self.offset += N::to_usize();
            unsafe {
                copy_nonoverlapping(src, dst, count);
            }
        } else {
            for i in 0..N::to_usize() {
                let idx = index_u8(i);
                let mask = mask_u8(i);

                let flag = self.bit()?;

                let byte = unsafe { arr.get_unchecked_mut(idx) };
                *byte = set_or_clear_u8(*byte, mask, flag);
            }
        }
        Ok(())
    }

    read_impl!(u8_gen, u8, U8);
    read_impl!(u16_gen, u16, U16);
    read_impl!(u32_gen, u32, U32);
    read_impl!(u64_gen, u64, U64);
    read_impl!(u128_gen, u128, U128);
}

pub struct BitWrite<'a> {
    buf: &'a mut [u8],
    offset: usize,
}

impl<'a> BitWrite<'a> {
    #[inline]
    pub fn new(buf: &'a mut [u8]) -> Self {
        // paranoid checking
        // up to 0x1fff_ffff on 32 bit system
        debug_assert!(buf.len() < std::usize::MAX / 8);
        Self { buf, offset: 0 }
    }

    #[inline(always)]
    pub fn is_aligned(&self) -> bool { self.offset % 8 == 0 }

    pub fn align(&mut self) -> Result<(), NoneError> {
        if index_u8(self.offset) < self.buf.len() {
            while !self.is_aligned() {
                unsafe { self.bit_unchecked(false); }
            }
            Ok(())
        } else {
            Err(NoneError)
        }
    }

    /// MAY PANICS
    pub fn align_ones(&mut self) -> Result<(), NoneError> {
        if index_u8(self.offset) < self.buf.len() {
            while !self.is_aligned() {
                unsafe { self.bit_unchecked(true); }
            }
            Ok(())
        } else {
            Err(NoneError)
        }
    }

    #[inline(always)]
    fn offset(&self) -> usize { self.offset }
    #[inline(always)]
    fn mask_index(&self) -> (u8, usize) {
        (mask_u8(self.offset), index_u8(self.offset))
    }

    #[inline]
    pub unsafe fn bit_unchecked(&mut self, flag: bool) {
        let (mask, idx) = self.mask_index();
        self.offset += 1;
        let byte = self.buf.get_unchecked_mut(idx);
        *byte = set_or_clear_u8(*byte, mask, flag);
    }

    #[inline]
    pub fn bit(&mut self, flag: bool) -> Result<(), NoneError> {
        let (mask, idx) = self.mask_index();
        let byte = self.buf.get_mut(idx)?;
        *byte = set_or_clear_u8(*byte, mask, flag);
        self.offset += 1;
        Ok(())
    }

    /// # Panics
    ///
    /// When `N > L * 8`. Only in debug mode.
    pub fn array<L, N>(&mut self, arr: &GenericArray<u8, L>) -> Result<(), NoneError>
        where L: ArrayLength<u8>, N: Unsigned,
    {
        debug_assert!(N::to_usize() <= L::to_usize() * 8);

        if N::to_usize() % 8 == 0 && self.offset % 8 == 0 {
            let count = N::to_usize() / 8;
            let a = self.offset / 8;
            let b = a + count;
            let src = arr.as_slice().as_ptr();
            let dst = self.buf.get_mut(a..b)?.as_mut_ptr();
            self.offset += N::to_usize();
            unsafe {
                copy_nonoverlapping(src, dst, count);
            }
        } else {
            for i in 0..N::to_usize() {
                let idx = index_u8(i);
                let mask = mask_u8(i);
                let flag = unsafe { arr.get_unchecked(idx) & mask != 0 };
                self.bit(flag)?;
            }
        }
        Ok(())
    }

    write_impl!(u8_gen, u8, U8);
    write_impl!(u16_gen, u16, U16);
    write_impl!(u32_gen, u32, U32);
    write_impl!(u64_gen, u64, U64);
    write_impl!(u128_gen, u128, U128);
}

#[inline(always)]
fn index_u8(bit: usize) -> usize { bit >> 3 }

#[inline(always)]
fn mask_u8(bit: usize) -> u8 { 1 << (bit & 0b111) }

/// Conditionally set or clear bits without branching for superscalar CPUs.
///
/// see: http://graphics.stanford.edu/~seander/bithacks.html#ConditionalSetOrClearBitsWithoutBranching
///
/// f - conditional flag
/// m - the bit mask
/// w - the word to modify:  if (f) w |= m; else w &= ~m;
#[inline(always)]
fn set_or_clear_u8(byte: u8, mask: u8, flag: bool) -> u8 {
    (byte & !mask) | ((-(flag as i8) as u8) & mask)
}

/// Calculates the number of bits required to serialize an integer in range [min,max].
///
/// - param min The minimum value.
/// - param max The maximum value.
///
/// returns The number of bits required to serialize the integer.
macro bits_required($min:expr, $max:expr) {
    size_of_val(&$min) - (max - min).leading_zeros()
}

macro zipzag_impl($enc:ident, $dec:ident, $i:ident, $u:ident) {
    /// Convert a signed integer to an unsigned integer with zig-zag encoding.
    /// 0,-1,+1,-2,+2... becomes 0,1,2,3,4 ...
    ///
    /// - param n The input value.
    ///
    /// returns The input value converted from signed to unsigned with zig-zag encoding.
    #[inline(always)]
    pub fn $enc(n: $i) -> $u {
        ((n << 1) ^ (n >> (size_of::<$i>() * 8 - 1))) as $u
    }

    /// Convert an unsigned integer to as signed integer with zig-zag encoding.
    /// 0,1,2,3,4... becomes 0,-1,+1,-2,+2...
    ///
    /// - param n The input value.
    ///
    /// returns The input value converted from unsigned to signed with zig-zag encoding.
    #[inline(always)]
    pub fn $dec(n: $u) -> $i {
        ((n >> 1) ^ (-((n & 1) as $i)) as $u) as $i
    }
}

zipzag_impl!(zipzag_encode8  , zipzag_decode8  , i8  , u8  );
zipzag_impl!(zipzag_encode16 , zipzag_decode16 , i16 , u16 );
zipzag_impl!(zipzag_encode32 , zipzag_decode32 , i32 , u32 );
zipzag_impl!(zipzag_encode64 , zipzag_decode64 , i64 , u64 );
zipzag_impl!(zipzag_encode128, zipzag_decode128, i128, u128);

#[test]
fn zipzag_encoding() {
    const MN: i8 = <i8>::min_value();
    const MX: i8 = <i8>::max_value();
    for v in MN..=MX {
        assert_eq!(v, zipzag_decode8(zipzag_encode8(v)));
    }

    for &v in &[0, -1, -2, -21, 34, 1, 2, 4] {
        assert_eq!(v as i8, zipzag_decode8(zipzag_encode8(v as i8)));
        assert_eq!(v as i16, zipzag_decode16(zipzag_encode16(v as i16)));
        assert_eq!(v as i32, zipzag_decode32(zipzag_encode32(v as i32)));
    }
}

#[test]
fn simple() {
    let buf = &mut (&mut [0; 5])[..];

    {
        let mut w = BitWrite::new(buf);
        for _ in 0..5 {
            for _ in 0..4 {
                w.bit(false).unwrap();
                w.bit(true).unwrap();
            }
        }
        w.bit(true).unwrap_err();
    }

    {
        let mut r = BitRead::new(buf);
        for _ in 0..5 {
            for _ in 0..4 {
                assert!(!r.bit().unwrap());
                assert!(r.bit().unwrap());
            }
        }
        r.bit().unwrap_err();
    }
}

#[test]
fn array32() {
    let buf = &mut (&mut [0; 4])[..];
    let v = 0b_10101010_10101010_10101010_10101010;

    {
        let mut w = BitWrite::new(buf);
        w.u32(v).unwrap();
        w.bit(true).unwrap_err();
    }

    {
        let mut r = BitRead::new(buf);
        assert_eq!(r.u32().unwrap(), v);
        r.bit().unwrap_err();
    }
}

#[test]
fn array32_noalign() {
    let buf = &mut (&mut [0; 5])[..];
    let v = 0b_10101010_10101010_10101010_10101010;

    {
        let mut w = BitWrite::new(buf);
        for _ in 0..2 {
            w.bit(false).unwrap();
            w.bit(true).unwrap();
        }
        w.u32(v).unwrap();
        w.align().unwrap();
        w.bit(true).unwrap_err();
    }

    {
        let mut r = BitRead::new(buf);
        for _ in 0..2 {
            assert!(!r.bit().unwrap());
            assert!(r.bit().unwrap());
        }
        assert_eq!(r.u32().unwrap(), v);
        for _ in 0..2 {
            assert!(!r.bit().unwrap());
            assert!(!r.bit().unwrap());
        }
        r.bit().unwrap_err();
    }
}
