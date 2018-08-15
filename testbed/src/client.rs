use std::time::Instant;
use specs::prelude::*;
use crate::{
    actor::Actor,
    lag::{Socket, LagNetwork},
    util::{duration_to_secs, secs_to_duration},
    prot::{Input, WorldState},
    consts::*,
};

/*
pub struct EWorld {
    pub world: World,
    pub dispatcher: Dispatcher<'static, 'static>,

    // Local representation of the entities.
    //entities: HashMap<usize, Actor>,

    // Unique ID of our entity.
    // Assigned by Server on connection.
}

impl EWorld {
    fn new(socket: Socket<Vec<WorldState>, Input>) -> Self {
        let mut world = World::new();
        world.register::<Actor>();

        world.add_resource(socket);
        world.add_resource(InputState {
            key_left: false,
            key_right: false,

            sequence: 1,
        });

        world.add_resource(Reconciliation::new());

        let dispatcher = DispatcherBuilder::new()
            .with(ProcessServerMessages, "ProcessServerMessages", &[])
            .with(ProcessInputs::new(), "ProcessInputs", &["ProcessServerMessages"])
            .with(InterpolateEntities, "InterpolateEntities", &["ProcessInputs"])
            .build();
        Self { world, dispatcher }
    }
}
*/

pub struct InputState {
    pub key_left: bool,
    pub key_right: bool,

    // Data needed for reconciliation.
    pub sequence: usize,
}

pub struct Client {
    pub world: World,
    pub dispatcher: Dispatcher<'static, 'static>,
    pub time: Instant,
    pub update_rate: f32,
}

pub struct Reconciliation {
    pending_inputs: Vec<Input>,
}

impl Reconciliation {
    fn new() -> Self {
        Self {
            pending_inputs: Vec::new(),
        }
    }

    pub fn non_acknowledged(&self) -> usize {
        self.pending_inputs.len()
    }

    fn save(&mut self, input: Input) {
        self.pending_inputs.push(input);
    }

    fn run(&mut self, entity: &mut Actor, state: &WorldState) {
        // Received the authoritative position
        // of self client's entity.
        entity.set_position(state.position);

        /*
        if !self.enabled {
            // Reconciliation is disabled,
            // so drop all the saved inputs.
            self.pending_inputs.clear();
            return;
        }
        */

        // Server Reconciliation.
        // Re-apply all the inputs not yet processed by the server.

        // Already processed.
        // Its effect is already taken into
        // account into the world update
        // we just got, so we can drop it.
        self.pending_inputs.retain(|i| i.sequence > state.last_processed_input);

        // Not processed by the server yet.
        // Re-apply it.
        for input in &self.pending_inputs {
            entity.apply_input(input.press_time);
        }
    }
}

impl Client {
    pub fn new(server: LagNetwork<Input>, network: LagNetwork<Vec<WorldState>>) -> Self {
        let socket = Socket::new(network, server);

        let mut world = World::new();
        world.register::<Actor>();

        world.add_resource(socket);
        world.add_resource(InputState {
            key_left: false,
            key_right: false,

            sequence: 1,
        });

        world.add_resource(Reconciliation::new());

        let dispatcher = DispatcherBuilder::new()
            .with(ProcessServerMessages, "ProcessServerMessages", &[])
            .with(ProcessInputs::new(), "ProcessInputs", &["ProcessServerMessages"])
            .with(InterpolateEntities, "InterpolateEntities", &["ProcessInputs"])
            .build();

        Self {
            world, dispatcher,
            time: Instant::now(),
            update_rate: CLIENT_UPDATE_RATE,
        }
    }

    pub fn status(&mut self) -> String {
        let me: Entity = *self.world.read_resource();
        let recv = self.socket().rx.recv_kbps();
        let count = self.world.read_resource::<Reconciliation>().non_acknowledged();
        format!("Another player [Arrows]\n recv bitrate: {}\n Update rate: {}/s\n ID: {}.\n Non-acknowledged inputs: {}",
            recv, self.update_rate, me.id(), count
        )
    }

    pub fn bind(&mut self, id: usize) {
        let me: Entity = unsafe { std::mem::transmute((id as u32, 1)) };
        self.world.add_resource(me);
    }

    pub fn fire(&mut self, fire: bool) {
        let me: Entity = *self.world.read_resource();
        let mut actors = self.world.write_storage::<Actor>();
        if let Some(node) = actors.get_mut(me).and_then(|e| e.node.as_mut()) {
            node.fire = fire
        }
    }

    /// Update Client state.
    pub fn update(&mut self) {
        let now = Instant::now();
        let dt = secs_to_duration(1.0 / self.update_rate);
        if self.time + dt <= now {
            self.time += dt;

            // Listen to the server.
            //self.process_server_messages();

            // Process inputs.
            //self.process_inputs();

            self.dispatcher.dispatch(&mut self.world.res);
            self.world.maintain();
        }
    }

    pub fn input_state(&mut self) -> shred::FetchMut<InputState> {
        self.world.write_resource::<InputState>()
    }

    pub fn socket(&mut self) -> shred::FetchMut<Socket<Vec<WorldState>, Input>> {
        self.world.write_resource::<Socket<Vec<WorldState>, Input>>()
    }

    pub fn key_left(&mut self, action: bool) {
        self.input_state().key_left = action;
    }

    pub fn key_right(&mut self, action: bool) {
        self.input_state().key_right = action;
    }
}

// Get inputs and send them to the server.
// If enabled, do client-side prediction.
pub struct ProcessInputs {
    last_processed: Instant,
}

impl ProcessInputs {
    fn new() -> Self {
        Self { last_processed: Instant::now() }
    }
}

#[derive(SystemData)]
pub struct ProcessInputsData<'a> {
    me: ReadExpect<'a, Entity>,
    input_state: WriteExpect<'a, InputState>,
    socket: WriteExpect<'a, Socket<Vec<WorldState>, Input>>,
    reconciliation: WriteExpect<'a, Reconciliation>,
    actors: WriteStorage<'a, Actor>,
}

impl<'a> System<'a> for ProcessInputs {
    type SystemData = ProcessInputsData<'a>;

    fn run(&mut self, mut data: Self::SystemData) {
        // Compute delta time since last update.
        let dt = {
            let now = Instant::now();
            let last = std::mem::replace(&mut self.last_processed, now);
            duration_to_secs(now - last)
        };

        let me: Entity = *data.me;
        let input = {
            let input_state = &mut data.input_state;

            // Package player's input.
            let mut input = Input {
                press_time: dt,
                sequence: input_state.sequence,
                entity_id: me.id() as usize,
            };

            if input_state.key_right {
                input.press_time *= 1.0;
            } else if input_state.key_left {
                input.press_time *= -1.0;
            } else {
                return; // Nothing interesting happened.
            };

            input_state.sequence += 1;
            input
        };

        // Do client-side prediction.
        data.actors.get_mut(me).unwrap().apply_input(input.press_time);
        // Send the input to the server.
        data.socket.send(input);
        // Save self input for later reconciliation.
        data.reconciliation.save(input);
    }
}

pub struct InterpolateEntities;

impl<'a> System<'a> for InterpolateEntities {
    type SystemData = (
        Entities<'a>,
        ReadExpect<'a, Entity>,
        WriteStorage<'a, Actor>,
    );

    fn run(&mut self, (entities, me, mut actors): Self::SystemData) {
        // Compute render time.
        let render_time = Instant::now() -
            secs_to_duration(1.0 / SERVER_UPDATE_RATE);

        // No point in interpolating self client's entity.
        let me = *me;
        let actors = (&*entities, &mut actors).join()
            .filter_map(|(e, a)| if e == me { None } else { Some(a) });

        for actor in actors {
            actor.interpolate(render_time);
        }
    }
}

// Process all messages from the server, i.e. world updates.
// If enabled, do server reconciliation.
pub struct ProcessServerMessages;

#[derive(SystemData)]
pub struct ProcessServerMessagesData<'a> {
        entities: Entities<'a>,
        reconciliation: WriteExpect<'a, Reconciliation>,
        socket: WriteExpect<'a, Socket<Vec<WorldState>, Input>>,
        me: ReadExpect<'a, Entity>,
        actors: WriteStorage<'a, Actor>,
        lazy: Read<'a, LazyUpdate>,
}

impl<'a> System<'a> for ProcessServerMessages {
    type SystemData = ProcessServerMessagesData<'a>;

    fn run(&mut self, mut data: Self::SystemData) {
        let now = Instant::now();
        let me = data.me.id() as usize;
        while let Some(message) = data.socket.recv() {
            // World state is a list of entity states.
            for state in &message {
                // If self is the first time we see self entity,
                // create a local representation.
                let id = state.entity_id;
                let position = state.position;

                let entity: Entity = unsafe { std::mem::transmute((id as u32, 1)) };
                let entity = if let Some(entity) = data.actors.get_mut(entity) {
                    entity
                } else {
                    let e = data.entities.create();
                    data.lazy.insert(e, Actor::spawn(id, position));
                    return;
                };

                if state.entity_id == me {
                    data.reconciliation.run(entity, state);
                } else {
                    // Received the position of an entity other than self client's.
                    // Add it to the position buffer.
                    entity.push_position(now, position);
                }
            }
        }
    }
}
