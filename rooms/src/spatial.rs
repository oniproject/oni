use fxhash::{FxHashSet as Set, FxHashMap as Map};
use num_traits::*;
use std::marker::PhantomData;

use crate::{
    iter2::Iter2,
    traits::Shim,
    entry::Entry,
};

pub struct SpatialHashMap<W, H, S: Shim> {
    map: Map<(S::Key, S::Key), Set<Entry<S>>>,
    pool: Vec<Set<Entry<S>>>,
    _marker: PhantomData<(W, H)>
}

impl<W, H, S> SpatialHashMap<W, H, S>
    where
        W: typenum::Unsigned,
        H: typenum::Unsigned,
        S: Shim,
{
    pub fn new() -> Self {
        Self {
            map: Map::default(),
            pool: Vec::new(),
            _marker: PhantomData,
        }
    }

    pub fn clear(&mut self) {
        for (_, mut set) in self.map.drain() {
            set.clear();
            self.pool.push(set);
        }
    }

    pub fn insert(&mut self, point: S::Vector, index: S::Index) -> bool {
        self.by_point_mut(point)
            .insert(Entry { index, point })
    }

    pub fn remove(&mut self, point: S::Vector, index: S::Index) -> bool {
        self.by_point_mut(point)
            .remove(&Entry { index, point })
    }

    pub fn iter_at(&mut self, point: S::Vector)
        -> impl Iterator<Item=S::Index> + '_
    {
        self.by_point_mut(point)
            .iter().map(|e| e.index)
    }

    #[inline]
    fn by_point_mut(&mut self, point: S::Vector) -> &mut Set<Entry<S>> {
        use std::collections::hash_map::Entry::*;
        let point = point.into();
        let key = S::hash2::<W, H>(point[0], point[1]);
        match self.map.entry(key) {
            Occupied(o) => o.into_mut(),
            Vacant(v) => v.insert(self.pool.pop().unwrap_or_default()),
        }
    }

    #[inline]
    fn by_key(&self, key: (S::Key, S::Key)) -> Option<&Set<Entry<S>>> {
        self.map.get(&key)
    }
}

impl<W, H, S> crate::SpatialIndex<S> for SpatialHashMap<W, H, S>
    where
        W: typenum::Unsigned,
        H: typenum::Unsigned,
        S: Shim,
{
    fn fill<I>(&mut self, pts: I)
        where I: Iterator<Item=(S::Index, S::Vector)>
    {
        for (index, point) in pts {
            self.by_point_mut(point)
                .insert(Entry { index, point });
        }
    }

    fn range<V>(&self, min: S::Vector, max: S::Vector, mut visitor: V)
        where V: FnMut(S::Index)
    {
        let (cx, cy) = {
            let (min, max) = (min.into(), max.into());
            let cx = S::hash2::<W, W>(min[0], max[0]);
            let cy = S::hash2::<H, H>(min[1], max[1]);
            (cx, cy)
        };

        let cells = Iter2::new(cx, cy)
            .filter_map(|key| self.by_key(key));

        for cell in cells {
            for index in cell.iter()
                .filter_map(|e| if S::in_rect(e.point, min, max) {
                    Some(e.index)
                } else {
                    None
                })
            {
                visitor(index)
            }
        }
    }

    fn within<V>(&self, center: S::Vector, radius: S::Scalar, mut visitor: V)
        where V: FnMut(S::Index)
    {
        // TODO use midpoint circle
        let (cx, cy) = {
            let center = center.into();
            let r_half = radius / (S::Scalar::one() + S::Scalar::one());
            let cx = S::hash2::<W, W>(center[0] - r_half, center[0] + r_half);
            let cy = S::hash2::<H, H>(center[1] - r_half, center[1] + r_half);

            let n = S::Key::one();
            let cx = (cx.0 - n, cx.1 + n);
            let cy = (cy.0 - n, cy.1 + n);
            (cx, cy)
        };

        let cells = Iter2::new(cx, cy)
            .filter_map(|key| self.by_key(key));

        for cell in cells {
            for index in cell.iter()
                .filter_map(|e| if S::in_circle(e.point, center, radius) {
                    Some(e.index)
                } else {
                    None
                })
            {
                visitor(index)
            }
        }
    }
}
