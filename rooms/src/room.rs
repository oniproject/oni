use specs::prelude::*;
use crate::{KDBush, AroundIndex, SpatialIndex, Tuple32, Shim};

#[derive(Component)]
#[storage(DenseVecStorage)]
pub struct Actor {
    pub position: [f32; 2],
    pub velocity: [f32; 2],
    pub room: Entity,
}

impl Actor {
    pub fn around<'a>(&self, index: &'a KDBush<Tuple32>) -> AroundIndex<'a, KDBush<Tuple32>, Tuple32> {
        index.around(self.position)
    }
}

#[derive(Component)]
#[storage(HashMapStorage)]
pub struct Room {
    index: KDBush<Tuple32>,
}

// FIXME: because KDBush not thread safe
unsafe impl<S: Shim> Send for KDBush<S> {}
unsafe impl<S: Shim> Sync for KDBush<S> {}

impl Room {
    pub fn new() -> Self {
        Self {
            index: KDBush::new(10),
        }
    }
}

pub struct RoomSystem {}

impl RoomSystem {
    pub fn new(world: &mut World) -> Self {
        world.register::<Room>();
        Self {
        }
    }
}

#[test]
fn room() {
    let mut world = World::new();
    let sys = RoomSystem::new(&mut world);

    let room = world.create_entity()
        .with(Room::new())
        .build();
}
