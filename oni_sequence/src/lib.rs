#[macro_use] extern crate serde_derive;

use std::{
    cmp::Ordering,
    fmt::Debug,
    hash::Hash,
};

use serde::{Serialize, Deserialize, Deserializer};

pub trait SequenceOps {
    fn next(self) -> Self;
    fn prev(self) -> Self;
}

#[derive(Serialize, Debug, Hash, Default, Eq, PartialEq, Clone, Copy)]
pub struct Sequence<T>(T)
    where T: Serialize + Debug + Hash + Default + Eq + Copy;

macro_rules! seq_impl {
    ($ty:ident) => {
        impl Sequence<$ty> {
            const HALF: $ty = $ty::max_value() / 2;
        }

        impl<'de> Deserialize<'de> for Sequence<$ty> {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
                where D: Deserializer<'de>
            {
                $ty::deserialize(deserializer)
                    .map(Sequence)
            }
        }

        impl SequenceOps for Sequence<$ty> {
            #[inline]
            fn next(self) -> Self { Sequence(self.0.wrapping_add(1)) }
            #[inline]
            fn prev(self) -> Self { Sequence(self.0.wrapping_sub(1)) }
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
