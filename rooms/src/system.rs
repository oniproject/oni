use specs::prelude::*;
use hibitset::DrainableBitSet;
use std::marker::PhantomData;
use crate::{
    index::{
        Around,
        SpatialIndex,
        Shim,
    },
    Replica,
    Actor,
    Room,
};

pub struct RoomSystem<S> {
    around: BitSet,
    _marker: PhantomData<S>
}

impl<S: Shim> RoomSystem<S> {
    pub fn new(world: &mut World) -> Self {
        world.register::<Replica<S>>();
        world.register::<Room<S>>();
        world.register::<Actor<S>>();
        Self {
            around: BitSet::new(),
            _marker: PhantomData,
        }
    }
}

impl<'a, S: Shim> System<'a> for RoomSystem<S> {
    type SystemData = (
        Entities<'a>,
        ReadStorage<'a, Actor<S>>,
        WriteStorage<'a, Replica<S>>,
        WriteStorage<'a, Room<S>>,
    );

    fn run(&mut self, (entities, actors, mut replica, mut rooms): Self::SystemData) {
        for (e_room, room) in (&*entities, &mut rooms).join() {
            let iter = (&*entities, &actors).join()
                .filter_map(|(e, a)| if a.room == e_room {
                    Some((e.id(), a.position))
                } else { None });
            room.index.fill(iter);
        }

        for (actor, rep) in (&actors, &mut replica).join() {
            if let Some(room) = rooms.get(actor.room) {
                room.around(actor.position)
                    .view(rep.view_range, |e| {
                        self.around.add(e);
                    });
                rep.extend(self.around.drain());
            }
        }
    }
}
