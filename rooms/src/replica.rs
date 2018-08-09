use fxhash::FxHashSet;
use specs::prelude::*;

#[derive(Component)]
#[storage(HashMapStorage)]
pub struct Replica<T>
    where T: Send + Sync + 'static
{
    old: FxHashSet<T>,
    new: FxHashSet<T>,
    nchange: Vec<T>,
    created: Vec<T>,
    removed: Vec<T>,
}

impl<T> Replica<T>
    where T: Send + Sync + Eq + std::hash::Hash + 'static
{
    pub fn new() -> Self {
        Self {
            new: FxHashSet::default(),
            old: FxHashSet::default(),
            created: Vec::new(),
            removed: Vec::new(),
            nchange: Vec::new(),
        }
    }

    pub fn all_unsorted(&self) -> impl Iterator<Item=&T> {
        self.new.iter()
    }

    pub fn created(&self) -> &[T] { &self.created }
    pub fn removed(&self) -> &[T] { &self.removed }
    pub fn nchange(&self) -> &[T] { &self.nchange }
}

impl<T> std::iter::Extend<T> for Replica<T>
    where T: Send + Sync + Copy + Eq + Ord + std::hash::Hash + 'static
{
    fn extend<I>(&mut self, new: I)
        where I: IntoIterator<Item=T>
    {
        std::mem::swap(&mut self.new, &mut self.old);
        self.new.clear();
        self.new.extend(new);

        self.nchange.clear();
        self.nchange.extend(self.new.intersection(&self.old).cloned());
        self.nchange.sort();

        self.created.clear();
        self.created.extend(self.new.difference(&self.old).cloned());
        self.created.sort();

        self.removed.clear();
        self.removed.extend(self.old.difference(&self.new).cloned());
        self.removed.sort();
    }
}

#[test]
fn replica() {
    let mut replica: Replica<usize> = Replica::new();

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
