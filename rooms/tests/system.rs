use specs::prelude::*;
use rooms::prelude32::*;

#[test]
fn simple() {
    let mut world = World::new();
    let sys = RoomSystem::new(&mut world);

    let room = Room::new();
    let room = world.create_entity()
        .with(room)
        .build();

    let e1 = world.create_entity()
        .with(Replica::new(View::Range(20.0, 10.0)))
        .with(Actor::new([0.0, 0.0], room))
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
