use num_traits::Float;
use std::hash::Hash;

pub mod brute;
pub mod kdbush;

pub trait Shim: 'static {
    type Index: Hash + Eq + Copy + 'static;
    type Scalar: Float + Send + Sync;
    type Vector:
        Into<[Self::Scalar; 2]> +
        From<[Self::Scalar; 2]> +
        Copy + Send + Sync;

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

pub trait SpatialIndex<S: Shim>: Sized {
    fn fill<I>(&mut self, pts: I) where I: Iterator<Item=(S::Index, S::Vector)>;

    fn range<V>(&self, min: S::Vector, max: S::Vector, visitor: V)
        where V: FnMut(S::Index);
    fn within<V>(&self, center: S::Vector, radius: S::Scalar, visitor: V)
        where V: FnMut(S::Index);

    fn around(&self, position: S::Vector) -> AroundIndex<Self, S> {
        AroundIndex { index: self, position }
    }
}

pub trait Around<S: Shim> {
    fn range<V: FnMut(S::Index)>(&self, w: S::Scalar, h: S::Scalar, visitor: V);
    fn within<V: FnMut(S::Index)>(&self, radius: S::Scalar, visitor: V);
}

pub struct AroundIndex<'a, I: SpatialIndex<S> + 'a, S: Shim> {
    index: &'a I,
    position: S::Vector,
}

impl<I: SpatialIndex<S>, S: Shim> Around<S> for AroundIndex<'a, I, S> {
    fn range<V: FnMut(S::Index)>(&self, w: S::Scalar, h: S::Scalar, visitor: V) {
        use num_traits::One;
        let two = S::Scalar::one() + S::Scalar::one();
        let (w, h) = (w / two, h / two);
        let [x, y]: [S::Scalar; 2] = self.position.into();
        self.index.range([x - w, y - h].into(), [x + w, y + h].into(), visitor);
    }
    fn within<V: FnMut(S::Index)>(&self, radius: S::Scalar, visitor: V) {
        self.index.within(self.position, radius, visitor);
    }
}
