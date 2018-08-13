use crate::{
    actor::Actor,
    lag::LagNetwork,
    net::{Input, WorldState},
    consts::*,
};

pub struct Connect {
    crate entity: Actor,
    network: LagNetwork<Vec<WorldState>>,
    last_processed_input: usize,
}

pub struct Server {
    // Connected clients and their entities.
    // Last processed input for each client.
    crate clients: Vec<Connect>,

    // Simulated network connection.
    crate network: LagNetwork<Input>,

    crate status: String,
}

impl Server {
    pub fn new(network: LagNetwork<Input>) -> Self {
        Self {
            network,
            clients: Vec::new(),
            status: String::new(),
        }
    }

    pub fn connect(&mut self, network: LagNetwork<Vec<WorldState>>) -> usize {
        let id = self.clients.len();
        // Set the initial state of the Entity (e.g. spawn point)
        let spawn_points = [4.0, 6.0];
        // Create a new Entity for self Client.
        let entity = Actor::new(id, spawn_points[id]);
        self.clients.push(Connect {
            entity,
            network,
            last_processed_input: 0,
        });
        id
    }

    pub fn update(&mut self) {
        self.process_inputs();
        self.send_world_state();
    }

    // Check whether self input seems to be valid (e.g. "make sense" according
    // to the physical rules of the World)
    fn validate_input(input: &Input) -> bool {
        input.press_time.abs() <= 1.0 / 40.0 * 1000.0
    }

    fn process_inputs(&mut self) {
        // Process all pending messages from clients.
        while let Some(message) = self.network.recv() {
            // Update the state of the entity, based on its input.
            // We just ignore inputs that don't look valid;
            // self is what prevents clients from cheating.
            if Self::validate_input(&message) {
                let id = message.entity_id;
                let client = &mut self.clients[id];
                client.entity.apply_input(message.press_time);
                client.last_processed_input = message.sequence;
            }
        }

        // Show some info.
        self.status = "Last acknowledged input:".to_string();
        for (i, client) in self.clients.iter().enumerate() {
            self.status += &format!(" [{}: #{}]", i, client.last_processed_input);
        }
    }

    /// Send the world state to all the connected clients.
    fn send_world_state(&mut self) {
        // Gather the state of the world.
        // In a real app, state could be filtered to avoid leaking data
        // (e.g. position of invisible enemies).
        let mut world_state = Vec::new();
        for c in &self.clients {
            world_state.push(WorldState {
                entity_id: c.entity.id(),
                position: c.entity.position(),
                last_processed_input: c.last_processed_input,
            });
        }

        // Broadcast the state to all the clients.
        for client in &mut self.clients {
            client.network.send(world_state.clone());
        }
    }
}
