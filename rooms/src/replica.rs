use fxhash::FxHashSet;
use specs::{prelude::*, world::Index};
use crate::index::{Shim, View};

#[derive(Component)]
#[storage(HashMapStorage)]
pub struct Replica<S: Shim> {
    all: FxHashSet<Index>,
    old: FxHashSet<Index>,
    nchange: Vec<Index>,
    created: Vec<Index>,
    removed: Vec<Index>,
    crate view_range: View<S>,
}

impl<S: Shim> Replica<S> {
    pub fn new(view_range: View<S>) -> Self {
        Self {
            view_range,
            all: FxHashSet::default(),
            old: FxHashSet::default(),
            created: Vec::new(),
            removed: Vec::new(),
            nchange: Vec::new(),
        }
    }

    pub fn all_unsorted(&self) -> impl Iterator<Item=Index> + '_ {
        self.all.iter().cloned()
    }

    pub fn created(&self) -> &[Index] { &self.created }
    pub fn removed(&self) -> &[Index] { &self.removed }
    pub fn nchange(&self) -> &[Index] { &self.nchange }

    pub fn populate_created<E: Extend<Index>>(&self, value: &mut E) {
        value.extend(self.all.intersection(&self.old).cloned());
    }
    pub fn populate_removed<E: Extend<Index>>(&self, value: &mut E) {
        value.extend(self.all.difference(&self.old).cloned());
    }
    pub fn populate_nchange<E: Extend<Index>>(&self, value: &mut E) {
        value.extend(self.old.difference(&self.all).cloned());
    }
}

impl<S: Shim> std::iter::Extend<Index> for Replica<S> {
    fn extend<I>(&mut self, new: I)
        where I: IntoIterator<Item=Index>
    {
        std::mem::swap(&mut self.all, &mut self.old);
        self.all.clear();
        self.all.extend(new);

        self.nchange.clear();
        self.created.clear();
        self.removed.clear();

        self.nchange.extend(self.all.intersection(&self.old).cloned());
        self.created.extend(self.all.difference(&self.old).cloned());
        self.removed.extend(self.old.difference(&self.all).cloned());

        self.nchange.sort();
        self.created.sort();
        self.removed.sort();
    }
}
