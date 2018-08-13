use std::{
    time::Instant,
    collections::HashMap,
};
use crate::{
    actor::Actor,
    lag::{Socket, LagNetwork},
    util::{duration_to_secs, secs_to_duration},
    net::{Input, WorldState},
    consts::*,
};

pub struct Client {
    // Simulated network connection.
    socket: Socket<Vec<WorldState>, Input>,

    // Local representation of the entities.
    crate entities: HashMap<usize, Actor>,

    // Unique ID of our entity.
    // Assigned by Server on connection.
    entity_id: usize,

    // Input state.
    crate key_left: bool,
    crate key_right: bool,

    // Data needed for reconciliation.
    client_side_prediction: bool,
    input_sequence_number: usize,

    reconciliation: Reconciliation,

    // Entity interpolation toggle.
    entity_interpolation: bool, // = true;

    last: Instant,

    crate status: String,
}

struct Reconciliation {
    pending_inputs: Vec<Input>,
    enabled: bool,
}

impl Reconciliation {
    fn non_acknowledged(&self) -> usize {
        self.pending_inputs.len()
    }

    fn save(&mut self, input: Input) {
        self.pending_inputs.push(input);
    }

    fn run(&mut self, entity: &mut Actor, state: &WorldState) {
        // Received the authoritative position
        // of self client's entity.
        entity.set_position(state.position);

        if !self.enabled {
            // Reconciliation is disabled,
            // so drop all the saved inputs.
            self.pending_inputs.clear();
            return;
        }

        // Server Reconciliation.
        // Re-apply all the inputs not yet processed by the server.

        // Already processed.
        // Its effect is already taken into
        // account into the world update
        // we just got, so we can drop it.
        self.pending_inputs
            .retain(|i| i.sequence > state.last_processed_input);

        // Not processed by the server yet.
        // Re-apply it.
        for input in &self.pending_inputs {
            entity.apply_input(input.press_time);
        }
    }
}

impl Client {
    pub fn new(server: LagNetwork<Input>, network: LagNetwork<Vec<WorldState>>) -> Self {
        Self {
            // Simulated network connection.
            /*
            server,
            network,
            */
            socket: Socket::new(network, server),

            // Local representation of the entities.
            entities: HashMap::new(),

            // Unique ID of our entity.
            // Assigned by Server on connection.
            entity_id: 0,

            // Input state.
            key_left: false,
            key_right: false,

            // Data needed for reconciliation.
            client_side_prediction: true,
            input_sequence_number: 1,

            reconciliation: Reconciliation {
                pending_inputs: Vec::new(),
                enabled: true,
            },

            // Entity interpolation toggle.
            entity_interpolation: true,

            status: String::new(),

            last: Instant::now(),
        }
    }

    pub fn bind(&mut self, id: usize) {
        self.entity_id = id;
    }

    /// Update Client state.
    pub fn update(&mut self) {
        // Listen to the server.
        self.process_server_messages();

        /*
        if self.entity_id == None {
            return;  // Not connected yet.
        }
        */

        // Process inputs.
        self.process_inputs();

        // Interpolate other entities.
        if self.entity_interpolation {
            self.interpolate_entities();
        }

        // Render the World.
        // TODO render_world(self.canvas, self.entities);

        // Show some info.
        self.status = format!("ID: {}. Non-acknowledged inputs: {}",
            self.entity_id,
            self.reconciliation.non_acknowledged());
    }

    // Get inputs and send them to the server.
    // If enabled, do client-side prediction.
    fn process_inputs(&mut self) {
        // Compute delta time since last update.
        let now = Instant::now();
        let last = std::mem::replace(&mut self.last, now);
        let dt = duration_to_secs(now - last);

        // Package player's input.
        let mut input = Input {
            press_time: dt,
            sequence: self.input_sequence_number,
            entity_id: self.entity_id,
        };

        if self.key_right {
            input.press_time *= 1.0;
        } else if self.key_left {
            input.press_time *= -1.0;
        } else {
            return; // Nothing interesting happened.
        };

        self.input_sequence_number += 1;

        // Do client-side prediction.
        if self.client_side_prediction {
            self.entities.get_mut(&self.entity_id)
                .unwrap()
                .apply_input(input.press_time);
        }

        // Send the input to the server.
        self.socket.send(input);

        // Save self input for later reconciliation.
        self.reconciliation.save(input);
    }


    // Process all messages from the server, i.e. world updates.
    // If enabled, do server reconciliation.
    fn process_server_messages(&mut self) {
        let now = Instant::now();
        while let Some(message) = self.socket.recv() {
            // World state is a list of entity states.
            for state in &message {
                // If self is the first time we see self entity,
                // create a local representation.
                let entity = self.entities.entry(state.entity_id)
                    .or_insert_with(|| Actor::new(state.entity_id, 0.0));

                if state.entity_id == self.entity_id {
                    self.reconciliation.run(entity, state);
                } else {
                    // Received the position of an entity other than self client's.
                    if !self.entity_interpolation {
                        // Entity interpolation is disabled
                        // - just accept the server's position.
                        entity.set_position(state.position);
                    } else {
                        // Add it to the position buffer.
                        entity.push_position(now, state.position);
                    }
                }
            }
        }
    }

    fn interpolate_entities(&mut self) {
        // Compute render time.
        let render_time = Instant::now() -
            secs_to_duration(1.0 / SERVER_UPDATE_RATE);

        // No point in interpolating self client's entity.
        let self_id = self.entity_id;
        self.entities.values_mut()
            .filter(|e| e.id() != self_id)
            .for_each(|e| e.interpolate(render_time));
    }
}
