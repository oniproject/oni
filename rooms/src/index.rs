mod brute;
mod kdbush;

use std::hash::{Hash, Hasher};

pub use self::brute::Brute;
pub use self::kdbush::KDBush;

crate struct Entry<S: Shim> {
    crate index: u32,
    crate point: [S; 2],
}

impl<S: Shim> From<(u32, [S; 2])> for Entry<S> {
    fn from(t: (u32, [S; 2])) -> Self {
        Self {
            index: t.0,
            point: t.1,
        }
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

impl Shim for f32 {}
impl Shim for f64 {}

pub trait Shim: num_traits::Float + Send + Sync + 'static {
    #[inline]
    fn in_rect(p: [Self; 2], min: [Self; 2], max: [Self; 2]) -> bool {
        p[0] >= min[0] && p[0] <= max[0] &&
        p[1] >= min[1] && p[1] <= max[1]
    }

    #[inline]
    fn in_circle(p: [Self; 2], center: [Self; 2], radius: Self) -> bool {
        Self::in_circle2(p, center, radius * radius)
    }

    #[inline]
    fn in_circle2(p: [Self; 2], center: [Self; 2], radius2: Self) -> bool {
        let dx = p[0] - center[0];
        let dy = p[1] - center[1];
        dx * dx + dy * dy <= radius2
    }
}

pub trait SpatialIndex<S: Shim>: Sized {
    fn fill<I>(&mut self, pts: I) where I: Iterator<Item=(u32, [S; 2])>;

    fn range<V>(&self, min: [S; 2], max: [S; 2], visitor: V)
        where V: FnMut(u32);
    fn within<V>(&self, center: [S; 2], radius: S, visitor: V)
        where V: FnMut(u32);

    fn around(&self, position: [S; 2]) -> AroundIndex<Self, S> {
        AroundIndex { index: self, position }
    }
}

#[derive(Clone, Copy, Debug)]
pub enum View<S: Shim> {
    Range(S, S),
    Within(S),
}

impl<S: Shim> From<[S; 2]> for View<S> {
    fn from(v: [S; 2]) -> Self {
        View::Range(v[0], v[1])
    }
}

impl<S: Shim> From<(S, S)> for View<S> {
    fn from(v: (S, S)) -> Self {
        View::Range(v.0, v.1)
    }
}

pub trait Around<S: Shim> {
    fn view<V: FnMut(u32)>(&self, view: View<S>, visitor: V);
}

pub struct AroundIndex<'a, I: SpatialIndex<S> + 'a, S: Shim> {
    index: &'a I,
    position: [S; 2],
}

impl<I: SpatialIndex<S>, S: Shim> Around<S> for AroundIndex<'a, I, S> {
    fn view<V: FnMut(u32)>(&self, view: View<S>, visitor: V) {
        match view {
            View::Range(w, h) => {
                let two = S::one() + S::one();
                let (w, h) = (w / two, h / two);
                let [x, y]: [S; 2] = self.position;
                self.index.range([x - w, y - h], [x + w, y + h], visitor);
            }
            View::Within(radius) => {
                self.index.within(self.position, radius, visitor);
            }
        }
    }
}
