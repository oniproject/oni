use std::hash::{Hash, Hasher};
use crate::Shim;

crate struct Entry<S: Shim> {
    crate index: S::Index,
    crate point: S::Vector,
}

impl<S: Shim> From<(S::Index, S::Vector)> for Entry<S> {
    fn from(t: (S::Index, S::Vector)) -> Self {
        Self {
            index: t.0,
            point: t.1,
        }
    }
}

impl<S: Shim> Entry<S> {
    #[inline(always)]
    crate fn axis(&self, axis: u8) -> S::Scalar {
        let p: [S::Scalar; 2] = self.point.into();
        unsafe { *p.get_unchecked(axis as usize) }
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
