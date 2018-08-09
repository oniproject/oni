use specs::prelude::*;
use crate::{
    KDBush,
    Around,
    SpatialIndex,
    Shim,
    Replica,
    util::Shim32,
};

#[derive(Component)]
#[storage(DenseVecStorage)]
pub struct Actor<S: Shim<Index=u32>> {
    pub position: S::Vector,
    pub velocity: S::Vector,
    pub view_range: S::Scalar,
    pub room: Entity,
    //pub _room: *const Room<S>,
}

impl<S> Actor<S>
    where S: Shim<Index=u32>
{
    pub fn around<'a>(&self, room: &'a Room<S>) -> impl Around<S> + 'a {
        room.index.around(self.position)
    }
}

#[derive(Component)]
#[storage(HashMapStorage)]
pub struct Room<S: Shim> {
    index: KDBush<S>,
}

// FIXME: because KDBush not thread safe
unsafe impl<S: Shim> Send for KDBush<S> {}
unsafe impl<S: Shim> Sync for KDBush<S> {}

impl<S: Shim<Index=u32>> Room<S> {
    pub fn new() -> Self {
        Self {
            index: KDBush::new(10),
        }
    }
}

pub struct RoomSystem {
}

impl RoomSystem {
    pub fn new(world: &mut World) -> Self {
        world.register::<Replica>();
        world.register::<Room<Shim32>>();
        world.register::<Actor<Shim32>>();
        Self {
        }
    }
}

impl<'a> System<'a> for RoomSystem {
    type SystemData = (
        Entities<'a>,
        ReadStorage<'a, Actor<Shim32>>,
        WriteStorage<'a, Room<Shim32>>,
        WriteStorage<'a, Replica>,
    );

    fn run(&mut self, (entities, actors, mut rooms, mut replica): Self::SystemData) {
        for (e_room, room) in (&*entities, &mut rooms).join() {
            let iter = (&*entities, &actors).join()
                .filter_map(|(e, a)| if a.room == e_room {
                    Some((e.id(), a.position))
                } else { None });
            room.index.fill(iter);
        }

        let mut around = std::collections::HashSet::new();
        for (actor, rep) in (&actors, &mut replica).join() {
            if let Some(room) = rooms.get(actor.room) {
                actor.around(room).within(actor.view_range, |e| {
                    around.insert(e);
                });
                rep.extend(around.drain());
            }
        }
    }
}

#[test]
fn simple() {
    let mut world = World::new();
    let sys = RoomSystem::new(&mut world);

    let room = world.create_entity()
        .with(Room::<Shim32>::new())
        .build();

    let e1 = world.create_entity()
        .with(Replica::new())
        .with(Actor::<Shim32> {
            position: [0.0, 0.0],
            velocity: [0.0, 0.0],
            view_range: 10.0,
            room,
        })
        .build();

    let mut dispatcher = DispatcherBuilder::new()
        .with(sys, "replica", &[])
        .build();

    dispatcher.dispatch(&world.res);

    let storage = world.read_storage::<Replica>();
    let re = storage.get(e1).unwrap();
    assert_eq!(re.created(), &[e1.id()]);

    {
        use std::iter::FromIterator;

        let created = BitSet::from_iter(re.created());
        let created: Vec<_> = (&*world.entities(), &created).join()
            .map(|(e, _)| e)
            .collect();

        assert_eq!(created, &[e1]);
    }
}
