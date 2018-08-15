use std::time::Instant;
use specs::prelude::*;
use crate::{
    actor::Actor,
    lag::{Socket, LagNetwork},
    util::{duration_to_secs, secs_to_duration},
    prot::{Input, WorldState},
    consts::*,
};

pub struct EWorld {
    crate world: World,

    // Local representation of the entities.
    //entities: HashMap<usize, Actor>,

    // Unique ID of our entity.
    // Assigned by Server on connection.
    crate entity_id: usize,
}

impl EWorld {
    fn new() -> Self {
        let mut world = World::new();
        world.register::<Actor>();

        Self { world, entity_id: 0 }
    }

    pub fn get_mut(&mut self, id: usize) -> Option<&mut Actor> {
        use specs::storage::UnprotectedStorage;

        //use std::iter::FromIterator;
        //let set = BitSet::from_iter(&[id as u32]);

        let entity: Entity = unsafe { std::mem::transmute((id as u32, 1)) };
        if self.world.is_alive(entity) {
            let mut storage = self.world.write_storage::<Actor>();
            unsafe {
                let r = storage.unprotected_storage_mut().get_mut(id as u32);
                Some(&mut *(r as *mut _))
            }
        } else {
            None
        }
    }

    fn get_or_insert(&mut self, id: usize) -> &mut Actor {
        //self.entities.entry(id).or_insert_with(|| Actor::new(id, 0.0))

        use specs::storage::UnprotectedStorage;

        let entity: Entity = unsafe { std::mem::transmute((id as u32, 1)) };
        if !self.world.is_alive(entity) {
            self.world.create_entity()
                .with(Actor::new(id, 0.0))
                .build();
            self.world.maintain();
        }

        let mut storage = self.world.write_storage::<Actor>();
        unsafe {
            let r = storage.unprotected_storage_mut().get_mut(id as u32);
            &mut *(r as *mut _)
        }
    }

    fn get_self_unwrap(&mut self) -> &mut Actor {
        use specs::storage::UnprotectedStorage;

        let id = self.entity_id;
        let mut storage = self.world.write_storage::<Actor>();
        unsafe {
            let r = storage.unprotected_storage_mut().get_mut(id as u32);
            &mut *(r as *mut _)
        }

        //self.entities.get_mut(&self.entity_id).unwrap()
    }
}

pub struct InputState {
    pub key_left: bool,
    pub key_right: bool,

    // Data needed for reconciliation.
    pub sequence: usize,
    pub prediction: bool,
}

pub struct Client {
    // Simulated network connection.
    crate socket: Socket<Vec<WorldState>, Input>,
    crate world: EWorld,
    crate input_state: InputState,
    crate reconciliation: Reconciliation,

    // Entity interpolation toggle.
    entity_interpolation: bool, // = true;

    last_process_input: Instant,
    time: Instant,
    update_rate: f32,
}

crate struct Reconciliation {
    pending_inputs: Vec<Input>,
    enabled: bool,
}

impl Reconciliation {
    crate fn non_acknowledged(&self) -> usize {
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
            socket: Socket::new(network, server),

            // Local representation of the entities.
            world: EWorld::new(),

            input_state: InputState {
                key_left: false,
                key_right: false,

                sequence: 1,
                prediction: true,
            },

            reconciliation: Reconciliation {
                pending_inputs: Vec::new(),
                enabled: true,
            },

            // Entity interpolation toggle.
            entity_interpolation: true,

            time: Instant::now(),
            last_process_input: Instant::now(),
            update_rate: CLIENT_UPDATE_RATE,
        }
    }

    pub fn status(&self) -> String {
        format!("Another player [Arrows]\n recv: {}\n\n ID: {}.\n Non-acknowledged inputs: {}",
            self.socket.rx.recv_kbps(),
            self.world.entity_id,
            self.reconciliation.non_acknowledged(),
        )
    }

    pub fn bind(&mut self, id: usize) {
        self.world.entity_id = id;
    }

    /// Update Client state.
    pub fn update(&mut self) {
        let now = Instant::now();
        let dt = secs_to_duration(1.0 / self.update_rate);
        if self.time + dt <= now {
            self.time += dt;

            // Listen to the server.
            self.process_server_messages();

            // Process inputs.
            self.process_inputs();

            // Interpolate other entities.
            if self.entity_interpolation {
                self.interpolate_entities();
            }
        }
    }

    // Get inputs and send them to the server.
    // If enabled, do client-side prediction.
    fn process_inputs(&mut self) {
        // Compute delta time since last update.
        let now = Instant::now();
        let last = std::mem::replace(&mut self.last_process_input, now);
        let dt = duration_to_secs(now - last);

        // Package player's input.
        let mut input = Input {
            press_time: dt,
            sequence: self.input_state.sequence,
            entity_id: self.world.entity_id,
        };

        if self.input_state.key_right {
            input.press_time *= 1.0;
        } else if self.input_state.key_left {
            input.press_time *= -1.0;
        } else {
            return; // Nothing interesting happened.
        };

        self.input_state.sequence += 1;

        // Do client-side prediction.
        if self.input_state.prediction {
            self.world.get_self_unwrap().apply_input(input.press_time);
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
        let me = self.world.entity_id;
        while let Some(message) = self.socket.recv() {
            // World state is a list of entity states.
            for state in &message {
                // If self is the first time we see self entity,
                // create a local representation.
                let entity = self.world.get_or_insert(state.entity_id);

                if state.entity_id == me {
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
        let self_id = self.world.entity_id;
        let mut storage = self.world.world.write_storage::<Actor>();
        (&mut storage).join()
            .filter(|e| e.id() != self_id)
            .for_each(|e| e.interpolate(render_time));
    }
}
