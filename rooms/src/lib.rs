//#![feature(associated_type_defaults)]

// ommoe - Oni Massively Multiplayer Online Engine

#[macro_use]
extern crate specs_derive;

mod entry;
mod spatial;
mod kdbush;

//mod room;
mod iter2;
mod traits;

pub use self::traits::{Shim, Tuple32, Tuple64};
pub use self::spatial::SpatialHashMap;
pub use self::kdbush::KDBush;

pub trait SpatialIndex<S: Shim> {
    fn range<V: FnMut(S::Index)>(&self, min: S::Vector, max: S::Vector, visitor: V);
    fn within<V: FnMut(S::Index)>(&self, center: S::Vector, radius: S::Scalar, visitor: V);
    //fn range_around<V: FnMut(u32)>(&self, id: u32, w: N, h: N, visitor: V);
    //fn within_around<V: FnMut(u32)>(&self, id: u32, radius: N, visitor: V);
}
