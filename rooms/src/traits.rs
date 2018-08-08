use num_traits::{Float, Num, NumAssignOps};
use typenum::Unsigned;
use std::hash::Hash;

pub trait Shim {
    type Index: Hash + Eq + Copy + 'static;
    type Key: Hash + Num + NumAssignOps + Ord + Copy;
    type Scalar: Float;
    type Vector:
        Into<[Self::Scalar; 2]> +
        From<[Self::Scalar; 2]> +
        Copy;

    fn hash<N: Unsigned>(s: Self::Scalar) -> Self::Key;

    #[inline]
    fn hash2<N, M>(n: Self::Scalar, m: Self::Scalar) -> (Self::Key, Self::Key)
        where
            N: Unsigned,
            M: Unsigned,
    {
        (Self::hash::<N>(n), Self::hash::<M>(m))
    }

    #[inline]
    fn in_rect(p: Self::Vector, min: Self::Vector, max: Self::Vector) -> bool {
        let (p, min, max) = (p.into(), min.into(), max.into());
        p[0] >= min[0] && p[0] <= max[0] &&
        p[1] >= min[1] && p[1] <= max[1]
    }

    #[inline]
    fn in_circle(
        p: Self::Vector, center: Self::Vector, radius: Self::Scalar,
    ) -> bool {
        Self::in_circle2(p, center, radius * radius)
    }

    #[inline]
    fn in_circle2(
        p: Self::Vector, center: Self::Vector, radius2: Self::Scalar,
    ) -> bool {
        let (p, center) = (p.into(), center.into());
        let dx = p[0] - center[0];
        let dy = p[1] - center[1];
        dx * dx + dy * dy <= radius2
    }
}

pub struct Tuple32;
impl Shim for Tuple32 {
    type Index = u32;
    type Key = i32;
    type Scalar = f32;
    type Vector = [Self::Scalar; 2];

    fn hash<N: Unsigned>(s: Self::Scalar) -> Self::Key { s as i32 / N::I32 }
}

pub struct Tuple64;
impl Shim for Tuple64 {
    type Index = u32;
    type Key = i32;
    type Scalar = f64;
    type Vector = [Self::Scalar; 2];

    fn hash<N: Unsigned>(s: Self::Scalar) -> Self::Key { s as i32 / N::I32 }
}
