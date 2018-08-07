use specs::prelude::*;

use typenum::U32;
use super::{SpatialHashMap, Tuple32};

#[derive(Component)]
#[storage(HashMapStorage)]
pub struct Room {
    hash: SpatialHashMap<U32, U32, Tuple32>
}

impl Room {
    pub fn new() -> Self {
        Self {
            hash: SpatialHashMap::new(),
        }
    }
}

pub struct RoomSystem {
}

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
