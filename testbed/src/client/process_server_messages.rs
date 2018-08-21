use std::time::Instant;
use std::net::SocketAddr;
use specs::prelude::*;
use oni::simulator::Socket;
use crate::{
    components::*,
    prot::*,
};

use super::{Reconciliation, StateBuffer};

// Process all messages from the server, i.e. world updates.
// If enabled, do server reconciliation.
pub struct ProcessServerMessages;

#[derive(SystemData)]
pub struct ProcessServerMessagesData<'a> {
    entities: Entities<'a>,
    reconciliation: WriteExpect<'a, Reconciliation>,
    server: ReadExpect<'a, SocketAddr>,
    socket: WriteExpect<'a, Socket>,
    me: ReadExpect<'a, Entity>,
    actors: WriteStorage<'a, Actor>,
    states: WriteStorage<'a, StateBuffer>,
    lazy: Read<'a, LazyUpdate>,
}

impl<'a> System<'a> for ProcessServerMessages {
    type SystemData = ProcessServerMessagesData<'a>;

    fn run(&mut self, mut data: Self::SystemData) {
        let now = Instant::now();
        let me = data.me.id() as usize;
        while let Some((message, addr)) = data.socket.recv_world() {
            assert_eq!(addr, *data.server);

            let last_processed_input = message.ack.0;

            // World state is a list of entity states.
            for m in &message.states {
                let id = unsafe { std::mem::transmute((m.entity_id as u32, 1)) };
                let actor = data.actors.get_mut(id);
                let state = data.states.get_mut(id);

                let (actor, state) = if let (Some(actor), Some(state)) = (actor, state) {
                    (actor, state)
                } else {
                    // If self is the first time we see self entity,
                    // create a local representation.
                    data.lazy.create_entity(&data.entities)
                        .from_server(m.entity_id)
                        .with(Actor::spawn(m.position))
                        .with(StateBuffer::new())
                        .build();
                    continue;
                };

                if m.entity_id == me as u16 {
                    data.reconciliation.reconciliation(
                        actor,
                        m.position,
                        last_processed_input,
                    );
                } else {
                    // Received the position of an entity other than self client's.
                    // Add it to the position buffer.
                    state.push_state(now, m);
                }
            }
        }
    }
}
