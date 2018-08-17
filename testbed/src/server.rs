use specs::prelude::*;
use specs::saveload::{Marker, MarkerAllocator};
use crate::{
    net_marker::*,
    actor::*,
    input::*,
    consts::*,
    util::*,
};

pub fn new_server(network: LagNetwork<Input>) -> Demo {
    let mut world = World::new();
    world.register::<Conn>();
    world.register::<Actor>();
    world.register::<NetMarker>();
    world.register::<LastProcessedInput>();
    world.add_resource(network);
    world.add_resource(NetNode::new(0..2));

    let dispatcher = DispatcherBuilder::new()
        .with(ProcessInputs, "ProcessInputs", &[])
        .with(SendWorldState, "SendWorldState", &["ProcessInputs"])
        .build();

    Demo::new(SERVER_UPDATE_RATE, world, dispatcher)
}

#[derive(Component)]
#[storage(DenseVecStorage)]
pub struct LastProcessedInput(pub usize);

#[derive(Component)]
#[storage(DenseVecStorage)]
pub struct Conn(pub LagNetwork<WorldState>);

pub struct ProcessInputs;

unsafe impl Send for ProcessInputs {}
unsafe impl Sync for ProcessInputs {}

impl<'a> System<'a> for ProcessInputs {
    type SystemData = (
        ReadExpect<'a, LagNetwork<Input>>,
        WriteStorage<'a, Actor>,
        WriteStorage<'a, LastProcessedInput>,
    );

    fn run(&mut self, (socket, mut actors, mut lpi): Self::SystemData) {
        use specs::storage::UnprotectedStorage;

        // Process all pending messages from clients.
        while let Some(message) = socket.recv() {
            // Update the state of the entity, based on its input.
            // We just ignore inputs that don't look valid;
            // self is what prevents clients from cheating.
            if validate_input(&message) {
                unsafe {
                    let id = message.entity_id as u32;
                    actors.unprotected_storage_mut().get_mut(id).apply_input(&message);
                    lpi.unprotected_storage_mut().get_mut(id).0 = message.sequence;
                }
            }
        }
    }
}

// Check whether self input seems to be valid (e.g. "make sense" according
// to the physical rules of the World)
fn validate_input(input: &Input) -> bool {
    input.press_time.abs() <= 1.0 / 40.0 * 1000.0
}

// Gather the state of the world.
// In a real app, state could be filtered to avoid leaking data
// (e.g. position of invisible enemies).
pub struct SendWorldState;

impl<'a> System<'a> for SendWorldState {
    type SystemData = (
        ReadStorage<'a, NetMarker>,
        WriteStorage<'a, Actor>,
        ReadStorage<'a, LastProcessedInput>,
        WriteStorage<'a, Conn>,
    );

    fn run(&mut self, mut data: Self::SystemData) {
        // Broadcast the state to all the clients.
        for (lpi, c) in (&data.2, &mut data.3).join() {
            let states: Vec<_> = (&data.0, &data.1)
                .join()
                // TODO: filter
                .map(|(e, a)| EntityState {
                    entity_id: e.id(),
                    position: a.position,
                    velocity: a.velocity,
                    rotation: a.rotation,
                })
                .collect();
            c.0.send(WorldState {
                states,
                last_processed_input: lpi.0,
            });
        }
    }
}
