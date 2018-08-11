use super::{Shim, Entry};

pub struct Brute<S: Shim> {
    data: Vec<Entry<S>>,
}

impl<S: Shim> super::SpatialIndex<S> for Brute<S> {
    fn fill<I>(&mut self, pts: I)
        where I: Iterator<Item=(u32, [S; 2])>
    {
        self.data.clear();
        self.data.extend(pts.map(Entry::from));
    }

    fn range<V>(&self, min: [S; 2], max: [S; 2], visitor: V)
        where V: FnMut(u32)
    {
        self.data.iter()
            .filter(|e| S::in_rect(e.point, min, max))
            .map(|e| e.index)
            .for_each(visitor)
    }

    fn within<V>(&self, center: [S; 2], radius: S, visitor: V)
        where V: FnMut(u32)
    {
        let r2 = radius * radius;
        self.data.iter()
            .filter(|e| S::in_circle2(e.point, center, r2))
            .map(|e| e.index)
            .for_each(visitor)
    }
}
