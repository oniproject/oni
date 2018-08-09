use crate::{Shim, Entry};

pub struct Brute<S: Shim> {
    data: Vec<Entry<S>>,
}

impl<S: Shim> crate::SpatialIndex<S> for Brute<S> {
    fn fill<I>(&mut self, pts: I)
        where I: Iterator<Item=(S::Index, S::Vector)>
    {
        self.data.clear();
        self.data.extend(pts.map(Entry::from));
    }

    fn range<V>(&self, min: S::Vector, max: S::Vector, visitor: V)
        where V: FnMut(S::Index)
    {
        self.data.iter()
            .filter(|e| S::in_rect(e.point, min, max))
            .map(|e| e.index)
            .for_each(visitor)
    }

    fn within<V>(&self, center: S::Vector, radius: S::Scalar, visitor: V)
        where V: FnMut(S::Index)
    {
        let r2 = radius * radius;
        self.data.iter()
            .filter(|e| S::in_circle2(e.point, center, r2))
            .map(|e| e.index)
            .for_each(visitor)
    }
}
