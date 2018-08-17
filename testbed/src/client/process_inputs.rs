use std::time::Instant;
use std::net::SocketAddr;
use specs::prelude::*;
use oni::simulator::Socket;
use crate::{
    prot::*,
    ai::*,
    actor::*,
    input::*,
    util::*,
    input_buf::SequenceOps,
};

use super::Reconciliation;

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
    me: ReadExpect<'a, Entity>,
    server: ReadExpect<'a, SocketAddr>,
    ai: Write<'a, Option<AI>>,
    stick: Write<'a, Option<Stick>>,
    reconciliation: WriteExpect<'a, Reconciliation>,
    socket: WriteExpect<'a, Socket>,
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

        let ai = data.ai.as_mut()
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
            };

            data.reconciliation.sequence =
                data.reconciliation.sequence.next();

            // Do client-side prediction.
            actor.apply_input(&input);
            // Send the input to the server.
            data.socket.send_input(input.clone(), *data.server);
            // Save self input for later reconciliation.
            data.reconciliation.save(input);
        }
    }
}
