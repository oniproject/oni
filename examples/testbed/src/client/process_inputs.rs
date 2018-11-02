use std::time::Instant;
use specs::prelude::*;
use oni::SimulatedSocket as Socket;
use oni_reliable::{Sequence, SequenceOps};
use crate::{
    components::*,
    prot::*,
    ai::*,
    input::*,
    util::*,
};

use super::{Reconciliation, Controller};

// Get inputs and send them to the server.
// Do client-side prediction.
pub struct ProcessInputs {
    last_processed: Instant,
    sender: InputSender,
}

impl ProcessInputs {
    pub fn new() -> Self {
        Self {
            last_processed: Instant::now(),
            sender: InputSender::new(),
        }
    }
    fn take_secs(&mut self) -> f32 {
        let now = Instant::now();
        let last = std::mem::replace(&mut self.last_processed, now);
        duration_to_secs(now - last)
    }
}

#[derive(SystemData)]
pub struct ProcessInputsData<'a> {
    node: ReadExpect<'a, NetNode>,

    ai: Option<Write<'a, AI>>,
    stick: Option<Write<'a, Stick>>,

    reconciliation: WriteExpect<'a, Reconciliation>,
    socket: WriteExpect<'a, oni::Client<Socket>>,

    actors: WriteStorage<'a, Actor>,

    last_frame: Read<'a, Sequence<u16>>,
}

impl<'a> System<'a> for ProcessInputs {
    type SystemData = ProcessInputsData<'a>;

    fn run(&mut self, mut data: Self::SystemData) {
        oni_trace::scope![client process inputs];

        decelerator!();
        data.socket.update();
        if !data.socket.is_connected() {
            debug!("disconnected");
            //println!("state: {:?}", data.socket.state());
            return;
        }

        // Compute delta time since last update.
        let press_delta = self.take_secs();

        let me = if let Some(me) = data.node.me() {
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

        let frame_ack = *data.last_frame;
        let seq = &mut data.reconciliation.sequence;

        let ai = data.ai.as_mut().and_then(|c| c.run(actor));
        let stick = data.stick.as_mut().and_then(|c| c.run(actor));

        let input = ai.or(stick).map(|stick| {
            actor.rotation = stick.rotation;

            // Package player's input.
            let stick: [f32; 2] = stick.translation.vector.into();
            let sequence = seq.fetch_next();
            let input = InputSample {
                frame_ack,

                press_delta,
                stick,
                rotation: actor.rotation.angle(),
                sequence,

                fire: actor.fire,
            };

            //trace!("send input: {:?}", input);
            oni_trace::instant!(json "input", json!({
                "frame_ack": frame_ack,

                "press_delta": press_delta,
                "stick": stick,
                "rotation": actor.rotation.angle(),
                "sequence": sequence,

                "fire": actor.fire,
            }));

            input
        });

        if let Some(input) = &input {
            // Do client-side prediction.
            actor.apply_input(input);
            // Save self input for later reconciliation.
            data.reconciliation.save(input.clone());
        }

        // Send the input to the server.
        let inputs: arrayvec::ArrayVec<_> = self.sender.send(input).collect();
        if !inputs.is_empty() {
            data.socket.send_client(Client::Input(inputs));
        }
    }
}
