use specs::prelude::*;
use crate::index::{KDBush, Shim, Around, SpatialIndex};

#[derive(Component)]
#[storage(HashMapStorage)]
pub struct Room<S: Shim> {
    crate index: KDBush<S>,
}

impl<S: Shim> Room<S> {
    pub fn new() -> Self {
        Self::with_node_size(10)
    }

    pub fn with_node_size(node_size: usize) -> Self {
        Self {
            index: KDBush::new(node_size),
        }
    }

    pub fn around<V: Into<[S; 2]>>(&self, position: V) -> impl Around<S> + '_ {
        self.index.around(position.into())
    }
}
