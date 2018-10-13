use std::hash::Hash;
use std::collections::HashMap;
use slotmap::{SlotMap, Key};

crate struct Store<T: Copy, K> {
    store: SlotMap<T>,
    connect: HashMap<(K, K), Key>,
    bind: HashMap<K, Key>,
}

impl<T, K> Store<T, K>
    where K: Hash + Copy + Eq, T: Copy
{
    crate fn new() -> Self {
        Self {
            store: SlotMap::new(),
            connect: HashMap::default(),
            bind: HashMap::default(),
        }
    }

    crate fn insert<U: Into<Option<K>>>(&mut self, from: K, to: U, data: T) {
        let key = self.store.insert(data);
        let key = if let Some(to) = to.into() {
            self.connect.insert((from, to), key)
        } else {
            self.bind.insert(from, key)
        };
        if let Some(key) = key {
            self.store.remove(key);
        }
    }

    crate fn remove<U: Into<Option<K>>>(&mut self, from: K, to: U) -> Option<T> {
        let to = to.into();
        let key = if let Some(to) = to {
            self.connect.get(&(from, to))
        } else {
            self.bind.get(&from)
        };
        let key = key.map(|&k| k).unwrap_or_default();
        self.store.remove(key)
    }

    crate fn any_find<U: Into<Option<K>>>(&self, from: K, to: U) -> Option<&T> {
        let to = to.into();
        let key = to
            .and_then(|to| self.connect.get(&(from, to)))
            .or_else(|| self.bind.get(&from))
            .map(|&k| k)
            .unwrap_or_default();
        self.store.get(key)
    }
}

#[test]
fn bind() {
    let mut store: Store<usize, u16> = Store::new();

    store.insert(5, None, 1234);

    assert_eq!(store.any_find(5, None).cloned(), Some(1234));
    assert_eq!(store.any_find(5, 7).cloned(), Some(1234));

    assert_eq!(store.remove(5, 7), None);
    assert_eq!(store.remove(5, None), Some(1234));
}

#[test]
fn connect() {
    let mut store: Store<usize, u16> = Store::new();

    store.insert(5, 7, 1234);

    assert_eq!(store.any_find(5, None).cloned(), None);
    assert_eq!(store.any_find(5, 7).cloned(), Some(1234));

    assert_eq!(store.remove(5, None), None);
    assert_eq!(store.remove(5, 7), Some(1234));
}

#[test]
fn all() {
    let mut store: Store<usize, u16> = Store::new();

    store.insert(5, None, 1234);
    store.insert(5, 7, 4321);

    assert_eq!(store.any_find(5, None).cloned(), Some(1234));
    assert_eq!(store.any_find(5, 7).cloned(), Some(4321));

    assert_eq!(store.remove(5, None).clone(), Some(1234));
    assert_eq!(store.remove(5, 7).clone(), Some(4321));
}
