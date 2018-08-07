use num_traits::{Float, ToPrimitive};
use num_traits::{Num, NumAssignOps};
use typenum::Unsigned;
use std::hash::Hash;

pub trait Shim {
    type Index: Hash + Eq + Copy + 'static;
    type Key: Hash + Num + NumAssignOps + Ord + Copy;
    type Scalar: Float + ToPrimitive;
    type Vector:
        Into<(Self::Scalar, Self::Scalar)> +
        From<(Self::Scalar, Self::Scalar)> +
        Copy;

    fn hash<N: Unsigned>(s: Self::Scalar) -> Self::Key;

    fn hash2<N, M>(n: Self::Scalar, m: Self::Scalar) -> (Self::Key, Self::Key)
        where
            N: Unsigned,
            M: Unsigned,
    {
        (Self::hash::<N>(n), Self::hash::<M>(m))
    }


    fn in_rect(point: Self::Vector, min: Self::Vector, max: Self::Vector) -> bool {
        let (point, min, max) = (point.into(), min.into(), max.into());
        point.0 >= min.0 && point.0 <= max.0 &&
        point.1 >= min.1 && point.1 <= max.1
    }

    fn in_circle(point: Self::Vector, center: Self::Vector, radius: Self::Scalar)
        -> bool
    {
        let (point, center) = (point.into(), center.into());
        let dx = point.0 - center.0;
        let dy = point.1 - center.1;
        dx * dx + dy * dy <= radius * radius
    }
}

pub struct Tuple32;
impl Shim for Tuple32 {
    type Index = u32;
    type Key = i32;
    type Scalar = f32;
    type Vector = (Self::Scalar, Self::Scalar);

    fn hash<N: Unsigned>(s: Self::Scalar) -> Self::Key { s as i32 / N::I32 }
}

pub struct Tuple64;
impl Shim for Tuple64 {
    type Index = u32;
    type Key = i32;
    type Scalar = f64;
    type Vector = (Self::Scalar, Self::Scalar);

    fn hash<N: Unsigned>(s: Self::Scalar) -> Self::Key { s as i32 / N::I32 }
}
