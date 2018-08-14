use crate::{
    actor::Actor,
    lag::LagNetwork,
    prot::{Input, WorldState},
    consts::*,
    util::*,
};

use std::time::Instant;
use specs::prelude::*;

#[derive(Component)]
#[storage(HashMapStorage)]
pub struct Connect {
    crate entity: Actor,
    crate network: LagNetwork<Vec<WorldState>>,
    crate last_processed_input: usize,
}

unsafe impl Send for Connect {}
unsafe impl Sync for Connect {}

pub struct Server {
    pub world: World,
    pub dispatcher: Dispatcher<'static, 'static>,
    pub time: Instant,
    pub update_rate: f32,
}

impl Server {
    pub fn new(network: LagNetwork<Input>) -> Self {
        let mut world = World::new();
        world.register::<Connect>();
        world.add_resource(network);

        let dispatcher = DispatcherBuilder::new()
            .with(ProcessInputs, "ProcessInputs", &[])
            .with(SendWorldState, "SendWorldState", &["ProcessInputs"])
            .build();
        Self {
            world,
            dispatcher,
            time: Instant::now(),
            update_rate: SERVER_UPDATE_RATE,
        }
    }

    pub fn update(&mut self) {
        let now = Instant::now();
        let dt = secs_to_duration(1.0 / self.update_rate);
        if self.time + dt <= now {
            self.time += dt;
            self.dispatcher.dispatch(&mut self.world.res);
            self.world.maintain();
        }
    }

    pub fn connect(&mut self, network: LagNetwork<Vec<WorldState>>) -> usize {
        // Set the initial state of the Entity (e.g. spawn point)
        let spawn_points = [4.0, 6.0];

        let e = self.world.create_entity().build();
        let id = e.id() as usize;

        let mut clients = self.world.write_storage::<Connect>();

        // Create a new Entity for self Client.
        clients.insert(e, Connect {
            entity: Actor::new(id, spawn_points[id]),
            network,
            last_processed_input: 0,
        }).unwrap();

        id
    }

    pub fn status(&self) -> String {
        let clients = self.world.read_storage::<crate::server::Connect>();
        let clients = (&clients).join().map(|c| c.last_processed_input);

        let recv = self.world.read_resource::<LagNetwork<Input>>().recv_kbps();
        let mut status = format!("Server\n recv bitrate:{}\n Update rate: {}/s\n Last acknowledged input:", recv, self.update_rate);
        for (i, last_processed_input) in clients.enumerate() {
            status += &format!("\n  [{}: #{}]", i, last_processed_input);
        }
        status
    }
}

pub struct ProcessInputs;

unsafe impl Send for ProcessInputs {}
unsafe impl Sync for ProcessInputs {}

impl<'a> System<'a> for ProcessInputs {
    type SystemData = (
        ReadExpect<'a, LagNetwork<Input>>,
        WriteStorage<'a, Connect>,
    );

    fn run(&mut self, (socket, mut clients, ): Self::SystemData) {
        use specs::storage::UnprotectedStorage;

        // Process all pending messages from clients.
        while let Some(message) = socket.recv() {
            // Update the state of the entity, based on its input.
            // We just ignore inputs that don't look valid;
            // self is what prevents clients from cheating.
            if validate_input(&message) {
                let id = message.entity_id as u32;
                let client = unsafe { clients.unprotected_storage_mut().get_mut(id) };
                client.entity.apply_input(message.press_time);
                client.last_processed_input = message.sequence;
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
    type SystemData = WriteStorage<'a, Connect>;

    fn run(&mut self, mut clients: Self::SystemData) {
        // Gather the state of the world.
        // In a real app, state could be filtered to avoid leaking data
        // (e.g. position of invisible enemies).
        let mut world_state = Vec::new();
        for c in (&clients).join() {
            world_state.push(WorldState {
                entity_id: c.entity.id(),
                position: c.entity.position(),
                last_processed_input: c.last_processed_input,
            });
        }

        // Broadcast the state to all the clients.
        for c in (&mut clients).join() {
            c.network.send(world_state.clone());
        }
    }
}
