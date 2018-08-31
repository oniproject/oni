use std::time::Instant;
use std::net::SocketAddr;
use specs::prelude::*;
use oni::{
    simulator::Socket,
    reliable::{Sequence, SequenceOps},
};
use crate::{
    components::*,
    prot::*,
    ai::*,
    input::*,
    util::*,
};

use super::{Reconciliation, Controller};

// Get inputs and send them to the server.
// If enabled, do client-side prediction.
pub struct ProcessInputs {
    last_processed: Instant,
}

impl ProcessInputs {
    pub fn new() -> Self {
        Self { last_processed: Instant::now() }
    }
}

#[derive(SystemData)]
pub struct ProcessInputsData<'a> {
    node: ReadExpect<'a, NetNode>,

    server: ReadExpect<'a, SocketAddr>,

    ai: Option<Write<'a, AI>>,
    stick: Option<Write<'a, Stick>>,

    reconciliation: WriteExpect<'a, Reconciliation>,
    socket: WriteExpect<'a, Socket>,

    actors: WriteStorage<'a, Actor>,

    last_frame: Read<'a, Sequence<u16>>,
}

impl<'a> System<'a> for ProcessInputs {
    type SystemData = ProcessInputsData<'a>;

    fn run(&mut self, mut data: Self::SystemData) {
        oni::trace::scope![client process inputs];

        // Compute delta time since last update.
        let dt = {
            let now = Instant::now();
            let last = std::mem::replace(&mut self.last_processed, now);
            duration_to_secs(now - last)
        };

        let me = if let Some(me) = data.node.me {
            me
        } else {
            debug!("disconnected");
            return;
        };
        let actor = if let Some(actor) = data.actors.get_mut(me) {
            actor
        } else {
            debug!("no actor: {:?}", me);
            return;
        };

        if let Some(stick) = data.stick.as_mut() {
            actor.fire = stick.get_fire();
        }

        let ai = data.ai.as_mut().and_then(|c| c.run(actor));
        let stick = data.stick.as_mut().and_then(|c| c.run(actor));

        if let Some(stick) = ai.or(stick) {
            actor.rotation = stick.rotation;

            let frame_ack = *data.last_frame;

            // Package player's input.
            let input = Input {
                frame_ack,

                press_delta: dt,
                stick: stick.translation.vector.clone(),
                rotation: actor.rotation.angle(),
                sequence: data.reconciliation.sequence.fetch_next(),

                fire: actor.fire,
            };

            //trace!("send input: {:?}", input);
            oni::trace::instant!(json "input", json!({
                "frame_ack": frame_ack,

                "press_delta": dt,
                "stick": stick.translation.vector.clone(),
                "rotation": actor.rotation.angle(),
                "sequence": data.reconciliation.sequence.fetch_next(),

                "fire": actor.fire,
            }));

            // Do client-side prediction.
            actor.apply_input(&input);
            // Send the input to the server.
            data.socket.send_client(Client::Input(input.clone()), *data.server);
            // Save self input for later reconciliation.
            data.reconciliation.save(input);
        }
    }
}
