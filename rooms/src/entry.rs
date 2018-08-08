use std::hash::{Hash, Hasher};
use crate::Shim;

pub struct Entry<S: Shim> {
    pub index: S::Index,
    pub point: S::Vector,
}

impl<S: Shim> Entry<S> {
    #[inline(always)]
    pub fn axis(&self, axis: u8) -> S::Scalar {
        let p: [S::Scalar; 2] = self.point.into();
        unsafe { *p.get_unchecked(axis as usize) }
    }

    #[inline(always)]
    pub fn x(&self) -> S::Scalar {
        let p: [S::Scalar; 2] = self.point.into();
        p[0]
    }

    #[inline(always)]
    pub fn y(&self) -> S::Scalar {
        let p: [S::Scalar; 2] = self.point.into();
        p[1]
    }
}

impl<S: Shim> Eq for Entry<S> {}

impl<S: Shim> PartialEq for Entry<S> {
    fn eq(&self, other: &Self) -> bool {
        self.index == other.index
    }
}

impl<S: Shim> Hash for Entry<S> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.index.hash(state);
    }
}
