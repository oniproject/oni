use std::time::Instant;
use std::net::SocketAddr;
use specs::prelude::*;
use specs::saveload::MarkerAllocator;
use oni::{
    simulator::Socket,
    reliable::Sequence,
};
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
    actors: WriteStorage<'a, Actor>,
    states: WriteStorage<'a, StateBuffer>,
    lazy: ReadExpect<'a, LazyUpdate>,
    node: WriteExpect<'a, NetNode>,

    last_frame: Write<'a, Sequence<u16>>,
}

impl<'a> System<'a> for ProcessServerMessages {
    type SystemData = ProcessServerMessagesData<'a>;

    fn run(&mut self, mut data: Self::SystemData) {
        oni::trace::scope![process server messages];

        decelerator!();

        let now = Instant::now();
        while let Some((message, addr)) = data.socket.recv_server() {
            match message {
                Server::Snapshot { ack, frame_seq, states } => {
                    assert_eq!(addr, *data.server);

                    let last_processed_input = ack.0;
                    *data.last_frame = frame_seq;

                    // World state is a list of entity states.
                    for m in &states {
                        let e = data.node.retrieve_entity_internal(m.entity_id() as u16);

                        let (e, state) = if let Some(e) = e {
                            (e, data.states.get_mut(e).unwrap())
                        } else {
                            let id = m.entity_id() as u16;
                            // If self is the first time we see self entity,
                            // create a local representation.
                            let e = data.lazy
                                .create_entity(&data.entities)
                                .from_server(id)
                                .with(Actor::spawn(m.position()))
                                .with(StateBuffer::new());
                            if id != 0 {
                                e.with(InterpolationMarker).build();
                            } else {
                                e.build();
                            }
                            continue;
                        };

                        if m.entity_id() == 0 {
                            let actor = data.actors.get_mut(e).unwrap();
                            data.reconciliation.reconciliation(
                                actor,
                                m.position(),
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
    }
}
