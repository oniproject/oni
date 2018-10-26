use std::{
    io,
    cmp::Ordering,
    mem::{replace, size_of},
};
use byteorder::{LE, ReadBytesExt, WriteBytesExt};
use serde::{
    ser::{Serialize, Serializer},
    de::{Deserialize, Deserializer},
};

pub trait SequenceIO: SequenceOps  {
    fn size_of() -> usize { size_of::<Self>() }

    fn write(self, buf: &mut [u8]) -> io::Result<()>;
    fn read(buf: &[u8]) -> io::Result<Self>;
}

impl SequenceIO for Sequence<u8> {
    #[inline]
    fn write(self, mut buf: &mut [u8]) -> io::Result<()> { buf.write_u8(self.0) }
    #[inline]
    fn read(mut buf: &[u8]) -> io::Result<Self> { buf.read_u8().map(Sequence) }
}

impl SequenceIO for Sequence<u16> {
    #[inline]
    fn write(self, mut buf: &mut [u8]) -> io::Result<()> { buf.write_u16::<LE>(self.0) }
    #[inline]
    fn read(mut buf: &[u8]) -> io::Result<Self> { buf.read_u16::<LE>().map(Sequence) }
}

impl SequenceIO for Sequence<u32> {
    #[inline]
    fn write(self, mut buf: &mut [u8]) -> io::Result<()> { buf.write_u32::<LE>(self.0) }
    #[inline]
    fn read(mut buf: &[u8]) -> io::Result<Self> { buf.read_u32::<LE>().map(Sequence) }
}

impl SequenceIO for Sequence<u64> {
    #[inline]
    fn write(self, mut buf: &mut [u8]) -> io::Result<()> { buf.write_u64::<LE>(self.0) }
    #[inline]
    fn read(mut buf: &[u8]) -> io::Result<Self> { buf.read_u64::<LE>().map(Sequence) }
}

impl SequenceIO for Sequence<u128> {
    #[inline]
    fn write(self, mut buf: &mut [u8]) -> io::Result<()> { buf.write_u128::<LE>(self.0) }
    #[inline]
    fn read(mut buf: &[u8]) -> io::Result<Self> { buf.read_u128::<LE>().map(Sequence) }
}

pub trait SequenceOps: Sized + Ord + Copy {
    const _HALF: Self;

    #[inline]
    fn next(self) -> Self { self.next_n(1) }
    #[inline]
    fn prev(self) -> Self { self.prev_n(1) }

    fn next_n(self, n: usize) -> Self;
    fn prev_n(self, n: usize) -> Self;

    fn to_usize(self) -> usize;

    fn into_index(self, cap: usize) -> usize {
        self.to_usize() % cap
    }

    fn fetch_next(&mut self) -> Self {
        let next = self.next();
        replace(self, next)
    }
    fn fetch_prev(&mut self) -> Self {
        let prev = self.prev();
        replace(self, prev)
    }
}

#[derive(Eq, PartialEq, Clone, Copy)]
pub struct Sequence<T: Eq + Copy>(T);

impl<T> Default for Sequence<T>
    where T: Default + Eq + Copy,
{
    fn default() -> Self {
        Sequence(T::default())
    }
}

impl<T> std::fmt::Debug for Sequence<T>
    where T: std::fmt::Debug + Eq + Copy,
{
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "Sequence({:?})", self.0)
    }
}

#[allow(clippy::derive_hash_xor_eq)]
impl<T> std::hash::Hash for Sequence<T>
    where T: std::hash::Hash + Eq + Copy,
{
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.0.hash(state)
    }
}

macro_rules! seq_impl {
    ($ty:ident) => {
        impl Sequence<$ty> {
            const HALF: $ty = $ty::max_value() / 2;
        }

        impl Serialize for Sequence<$ty> {
            #[inline]
            fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
                self.0.serialize(serializer)
            }
        }

        impl<'de> Deserialize<'de> for Sequence<$ty> {
            #[inline]
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
                where D: Deserializer<'de>
            {
                $ty::deserialize(deserializer)
                    .map(Sequence)
            }
        }

        impl SequenceOps for Sequence<$ty> {
            const _HALF: Self = Sequence($ty::max_value() / 2);

            #[inline]
            fn next_n(self, n: usize) -> Self {
                Sequence(self.0.wrapping_add(n as $ty))
            }
            #[inline]
            fn prev_n(self, n: usize) -> Self {
                Sequence(self.0.wrapping_sub(n as $ty))
            }

            #[inline]
            fn to_usize(self) -> usize { self.0 as usize }
        }

        impl From<$ty> for Sequence<$ty> {
            #[inline]
            fn from(v: $ty) -> Self { Sequence(v) }
        }

        impl Into<$ty> for Sequence<$ty> {
            #[inline]
            fn into(self) -> $ty { self.0 }
        }

        impl PartialOrd for Sequence<$ty> {
            #[inline]
            fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
                Some(self.cmp(other))
            }
        }

        impl Ord for Sequence<$ty> {
            #[inline]
            fn cmp(&self, other: &Self) -> Ordering {
                if self.0 == other.0 {
                    Ordering::Equal
                } else {
                    if self.0.wrapping_sub(other.0) < Self::HALF {
                        Ordering::Greater
                    } else {
                        Ordering::Less
                    }
                }
            }
        }
    }
}

seq_impl!(u8);
seq_impl!(u16);
seq_impl!(u32);
seq_impl!(u64);
seq_impl!(u128);

#[test]
fn sequence_u8() {
    let tests: &[(u8, u8, u8)] = &[
        (0x01, 0x00, 0xFF),
        (0x02, 0x01, 0x00),
        (0x03, 0x02, 0x01),
        (0xFF, 0xFE, 0xFD),
        (0x00, 0xFF, 0xFE),
    ];

    for (a, b, c) in tests.into_iter().cloned() {
        for i in 0..=0xFF {
            let ia = a.wrapping_add(i);
            let ib = b.wrapping_add(i);
            let ic = c.wrapping_add(i);

            let a = Sequence::from(ia);
            let b = Sequence::from(ib);
            let c = Sequence::from(ic);

            let _ia: u8 = a.into();
            let _ib: u8 = b.into();
            let _ic: u8 = c.into();

            assert_eq!(_ia, ia);
            assert_eq!(_ib, ib);
            assert_eq!(_ic, ic);

            assert_eq!(a.prev(), b);
            assert_eq!(b.prev(), c);

            assert_eq!(b.next(), a);
            assert_eq!(c.next(), b);

            assert_eq!(a.prev().prev(), c);
            assert_eq!(c.next().next(), a);

            assert_eq!(a.prev().next(), a);
            assert_eq!(b.prev().next(), b);
            assert_eq!(c.prev().next(), c);

            assert_eq!(a, a);
            assert_eq!(b, b);
            assert_eq!(c, c);

            assert!(a > b, "a > b: {:?} {:?}", a, b);
            assert!(b > c, "b > c: {:?} {:?}", b, c);
            assert!(a > c, "a > c: {:?} {:?}", a, c);

            assert!(b < a, "b < a: {:?} {:?}", b, a);
            assert!(c < b, "c < b: {:?} {:?}", c, b);
            assert!(c < a, "c < a: {:?} {:?}", c, a);
        }
    }
}
