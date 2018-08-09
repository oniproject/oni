#![feature(
    associated_type_defaults,
    decl_macro,
    macro_at_most_once_rep,
    macro_vis_matcher,
)]

// ommoe - Oni Massively Multiplayer Online Engine

#[macro_use]
extern crate specs_derive;

mod entry;
mod spatial;
mod kdbush;

mod iter2;
mod traits;

mod room;

//mod actors;
//mod explosion;
mod replica;

crate use self::entry::Entry;
pub use self::traits::{Shim, Tuple32, Tuple64};
pub use self::spatial::SpatialHashMap;
pub use self::kdbush::KDBush;

pub trait SpatialIndex<S: Shim>: Sized {
    fn fill<I>(&mut self, pts: I) where I: Iterator<Item=(S::Index, S::Vector)>;

    fn range<V: FnMut(S::Index)>(&self, min: S::Vector, max: S::Vector, visitor: V);
    fn within<V: FnMut(S::Index)>(&self, center: S::Vector, radius: S::Scalar, visitor: V);

    fn around(&self, position: S::Vector) -> AroundIndex<Self, S> {
        AroundIndex { index: self, position }
    }
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

pub trait Around<S: Shim> {
    fn range<V: FnMut(S::Index)>(&self, w: S::Scalar, h: S::Scalar, visitor: V);
    fn within<V: FnMut(S::Index)>(&self, radius: S::Scalar, visitor: V);
}
