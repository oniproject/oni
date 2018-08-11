use specs::prelude::*;
use crate::index::Shim;

#[derive(Component)]
#[storage(DenseVecStorage)]
pub struct Actor<S: Shim> {
    crate position: [S; 2],
    crate room: Entity,
}

impl<S: Shim> Actor<S> {
    pub fn new<V: Into<[S; 2]>>(position: V, room: Entity) -> Self {
        Self { room, position: position.into() }
    }
    pub fn set_position<V: Into<[S; 2]>>(&mut self, position: V) {
        self.position = position.into();
    }
}
