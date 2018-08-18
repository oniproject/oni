use specs::{
    prelude::*,
    saveload::Marker,
};
use oni::simulator::Socket;
use crate::{
    components::*,
    prot::*,
    prot::Endpoint,
    consts::*,
    util::*,
};

pub fn new_server(network: Socket) -> Demo {
    let mut world = World::new();
    world.register::<Conn>();
    world.register::<Actor>();
    world.register::<NetMarker>();
    world.register::<LastProcessedInput>();

    world.add_resource(network);
    world.add_resource(NetNode::new(0..2));

    Demo::new(SERVER_UPDATE_RATE, world, DispatcherBuilder::new()
        .with(ProcessInputs, "ProcessInputs", &[])
        .with(SendWorldState, "SendWorldState", &["ProcessInputs"]))
}

pub struct ProcessInputs;

unsafe impl Send for ProcessInputs {}
unsafe impl Sync for ProcessInputs {}

impl<'a> System<'a> for ProcessInputs {
    type SystemData = (
        ReadExpect<'a, Socket>,
        WriteStorage<'a, Actor>,
        WriteStorage<'a, LastProcessedInput>,
        ReadExpect<'a, NetNode>,
    );

    fn run(&mut self, (socket, mut actors, mut lpi, node): Self::SystemData) {
        // Process all pending messages from clients.
        while let Some((message, addr)) = socket.recv_input() {
            // Update the state of the entity, based on its input.
            // We just ignore inputs that don't look valid;
            // self is what prevents clients from cheating.
            if validate_input(&message) {
                let entity = node.by_addr.get(&addr).cloned().unwrap();
                actors.get_mut(entity).unwrap().apply_input(&message);
                lpi.get_mut(entity).unwrap().0 = message.sequence;
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

#[derive(SystemData)]
pub struct SendWorldStateData<'a> {
    socket: ReadExpect<'a, Socket>,
    mark: ReadStorage<'a, NetMarker>,
    actors: WriteStorage<'a, Actor>,
    lpi: ReadStorage<'a, LastProcessedInput>,
    addr: WriteStorage<'a, Conn>,
}

impl<'a> System<'a> for SendWorldState {
    type SystemData = SendWorldStateData<'a>;

    fn run(&mut self, mut data: Self::SystemData) {
        // Broadcast the state to all the clients.
        for (lpi, addr) in (&data.lpi, &mut data.addr).join() {
            let states: Vec<_> = (&data.mark, &data.actors)
                .join()
                // TODO: filter
                .map(|(e, a)| EntityState {
                    entity_id: e.id(),
                    position: a.position,
                    //velocity: a.velocity,
                    rotation: a.rotation.angle(),
                })
                .collect();

            data.socket.send_world(WorldState {
                states,
                last_processed_input: lpi.0,
            }, addr.0);
        }
    }
}
