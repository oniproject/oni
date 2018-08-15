use specs::prelude::*;
use crate::{
    actor::Actor,
    prot::{Input, WorldState},
    consts::*,
    util::*,
};

pub fn new_server(network: LagNetwork<Input>) -> Demo {
    let mut world = World::new();
    world.register::<Conn>();
    world.register::<Actor>();
    world.register::<LastProcessedInput>();
    world.add_resource(network);

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
pub struct Conn(pub LagNetwork<Vec<WorldState>>);

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
                    actors.unprotected_storage_mut().get_mut(id).apply_input(message.press_time);
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

pub struct SendWorldState;

impl<'a> System<'a> for SendWorldState {
    type SystemData = (
        WriteStorage<'a, Actor>,
        WriteStorage<'a, LastProcessedInput>,
        WriteStorage<'a, Conn>,
    );

    fn run(&mut self, mut data: Self::SystemData) {
        // Gather the state of the world.
        // In a real app, state could be filtered to avoid leaking data
        // (e.g. position of invisible enemies).
        let mut world_state = Vec::new();
        for (a, lpi) in (&data.0, &data.1).join() {
            world_state.push(WorldState {
                entity_id: a.id(),
                position: a.position(),
                last_processed_input: lpi.0,
            });
        }

        // Broadcast the state to all the clients.
        for c in (&mut data.2).join() {
            c.0.send(world_state.clone());
        }
    }
}
