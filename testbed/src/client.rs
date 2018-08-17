use std::time::Instant;
use specs::prelude::*;
use specs::saveload::{Marker, MarkerAllocator};
use crate::{
    net_marker::*,
    ai::*,
    actor::*,
    input::*,
    consts::*,
    util::*,
};

pub fn new_client(server: LagNetwork<Input>, network: LagNetwork<WorldState>, is_ai: bool) -> Demo {
    let socket = Socket::new(network, server);

    let mut world = World::new();
    world.register::<Actor>();
    world.register::<NetMarker>();

    world.add_resource(socket);
    world.add_resource(Reconciliation::new());
    world.add_resource(NetNode::new(0..2));
    if is_ai {
        world.add_resource::<Option<AI>>(Some(AI::new()));
        world.add_resource::<Option<Stick>>(None);
        //unimplemented!()
    } else {
        world.add_resource::<Option<AI>>(None);
        world.add_resource::<Option<Stick>>(Some(Stick::default()));
    }

    let dispatcher = DispatcherBuilder::new()
        .with(ProcessServerMessages, "ProcessServerMessages", &[])
        .with(ProcessInputs::new(), "ProcessInputs", &["ProcessServerMessages"])
        .with(InterpolateEntities, "InterpolateEntities", &["ProcessInputs"])
        .build();

    Demo::new(CLIENT_UPDATE_RATE, world, dispatcher)
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
    ai: Write<'a, Option<AI>>,
    stick: Write<'a, Option<Stick>>,
    reconciliation: WriteExpect<'a, Reconciliation>,
    socket: WriteExpect<'a, Socket<WorldState, Input>>,
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
        let actor = if let Some(actor) = data.actors.get_mut(me) {
            actor
        } else {
            return;
        };

        let mut ai = data.ai.as_mut()
            .and_then(|ai| ai.gen_stick(actor.position));

        let stick = data.stick.as_mut()
            .and_then(|s| s.take_updated()) // if nothing interesting happened.
            .or(ai);

        if let Some(stick) = stick {
            // Package player's input.
            let input = Input {
                press_time: dt,
                stick: stick.clone(),
                rotation: actor.rotation.angle(),
                sequence: data.reconciliation.sequence,
                entity_id: me.id() as usize,
            };

            data.reconciliation.sequence += 1;

            // Do client-side prediction.
            actor.apply_input(&input);
            // Send the input to the server.
            data.socket.send(input.clone());
            // Save self input for later reconciliation.
            data.reconciliation.save(input);
        }
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

        let me = *me;
        let actors = (&*entities, &mut actors).join()
            // No point in interpolating self client's entity.
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
    socket: WriteExpect<'a, Socket<WorldState, Input>>,
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
            for state in &message.states {
                // If self is the first time we see self entity,
                // create a local representation.
                let entity = unsafe { std::mem::transmute((state.entity_id as u32, 1)) };
                let entity = if let Some(entity) = data.actors.get_mut(entity) {
                    entity
                } else {
                    data.lazy.create_entity(&data.entities)
                        .from_server(state.entity_id)
                        .with(Actor::spawn(state.position))
                        .build();
                    continue;
                };

                if state.entity_id == me as u16 {
                    data.reconciliation.reconciliation(
                        entity,
                        state.position,
                        message.last_processed_input);
                } else {
                    // Received the position of an entity other than self client's.
                    // Add it to the position buffer.
                    entity.push_state(now, state);
                }
            }
        }
    }
}
