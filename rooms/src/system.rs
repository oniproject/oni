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
    Spawned,
    Position,
    Room,
};

pub struct MultiSystem<S> {
    around: BitSet,
    _marker: PhantomData<S>
}

impl<S: Shim> MultiSystem<S> {
    pub fn new(world: &mut World) -> Self {
        world.register::<Replica<S>>();
        world.register::<Room<S>>();
        world.register::<Position<S>>();
        world.register::<Spawned>();
        Self {
            around: BitSet::new(),
            _marker: PhantomData,
        }
    }
}

impl<'a, S: Shim> System<'a> for MultiSystem<S> {
    type SystemData = (
        Entities<'a>,
        ReadStorage<'a, Position<S>>,
        ReadStorage<'a, Spawned>,
        WriteStorage<'a, Replica<S>>,
        WriteStorage<'a, Room<S>>,
    );

    fn run(&mut self, (entities, pos, spawn, mut replica, mut rooms): Self::SystemData) {
        for (e_room, room) in (&*entities, &mut rooms).join() {
            let iter = (&*entities, &spawn, &pos).join()
                .filter_map(|(e, s, p)| if s.room == e_room {
                    Some((e.id(), p.position))
                } else { None });
            room.index.fill(iter);
        }

        for (pos, spawn, rep) in (&pos, &spawn, &mut replica).join() {
            if let Some(room) = rooms.get(spawn.room) {
                room.around(pos.position)
                    .view(rep.view_range, |e| {
                        self.around.add(e);
                    });
                rep.extend(self.around.drain());
            }
        }
    }
}


pub struct SingleSystem<S> {
    around: BitSet,
    _marker: PhantomData<S>
}

impl<S: Shim> SingleSystem<S> {
    pub fn new(world: &mut World) -> Self {
        world.register::<Replica<S>>();
        world.register::<Position<S>>();
        Self {
            around: BitSet::new(),
            _marker: PhantomData,
        }
    }
}

impl<'a, S: Shim> System<'a> for SingleSystem<S> {
    type SystemData = (
        Entities<'a>,
        WriteExpect<'a, Room<S>>,
        ReadStorage<'a, Position<S>>,
        WriteStorage<'a, Replica<S>>,
    );

    fn run(&mut self, (entities, mut room, pos, mut replica): Self::SystemData) {
        let iter = (&*entities, &pos).join()
            .map(|(e, p)| (e.id(), p.position));
        room.index.fill(iter);

        for (pos, rep) in (&pos, &mut replica).join() {
            room.around(pos.position)
                .view(rep.view_range, |e| {
                    self.around.add(e);
                });
            rep.extend(self.around.drain());
        }
    }
}
