use fxhash::FxHashSet;
use specs::prelude::*;
use specs::world::Index;

#[derive(Component)]
#[storage(HashMapStorage)]
pub struct Replica {
    old: FxHashSet<Index>,
    all: FxHashSet<Index>,
    nchange: Vec<Index>,
    created: Vec<Index>,
    removed: Vec<Index>,
}

impl Replica {
    pub fn new() -> Self {
        Self {
            old: FxHashSet::default(),
            all: FxHashSet::default(),
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
    pub fn populate_common<E: Extend<Index>>(&self, value: &mut E) {
        value.extend(self.old.difference(&self.all).cloned());
    }
}

impl std::iter::Extend<Index> for Replica {
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

#[test]
fn calc() {
    let mut replica = Replica::new();

    {
        replica.extend(vec![1, 2, 3]);
        assert_eq!(&replica.created(), &[1, 2, 3]);
        assert_eq!(&replica.removed(), &[]);
        assert_eq!(&replica.nchange(), &[]);
    }

    {
        replica.extend(vec![4, 2, 3, 4]);
        assert_eq!(&replica.created(), &[4]);
        assert_eq!(&replica.removed(), &[1]);
        assert_eq!(&replica.nchange(), &[2, 3]);
    }

    {
        replica.extend(vec![4, 2, 3, 4]);
        assert_eq!(&replica.created(), &[]);
        assert_eq!(&replica.removed(), &[]);
        assert_eq!(&replica.nchange(), &[2, 3, 4]);
    }

    {
        replica.extend(std::iter::empty());
        assert_eq!(&replica.created(), &[]);
        assert_eq!(&replica.removed(), &[2, 3, 4]);
        assert_eq!(&replica.nchange(), &[]);
    }
}
