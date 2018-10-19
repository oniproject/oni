use nalgebra::Point2;
use oni_reliable::Sequence;
use crate::{
    components::Actor,
    prot::*,
};

// Data needed for reconciliation.
pub struct Reconciliation {
    pub sequence: Sequence<u8>,
    pub pending_inputs: Vec<InputSample>,
}

impl Reconciliation {
    pub fn new() -> Self {
        Self {
            sequence: Sequence::from(1),
            pending_inputs: Vec::new(),
        }
    }

    pub fn non_acknowledged(&self) -> usize {
        self.pending_inputs.len()
    }

    pub fn save(&mut self, input: InputSample) {
        self.pending_inputs.push(input);
    }

    pub fn reconciliation(
        &mut self, actor: &mut Actor, position: Point2<f32>,
        input_ack: Sequence<u8>,
    ) {
        // Received the authoritative position
        // of self client's entity.
        actor.position = position;

        if false {
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
        self.pending_inputs.retain(|i| i.sequence > input_ack);

        // Not processed by the server yet.
        // Re-apply it.
        for input in &self.pending_inputs {
            actor.apply_input(input);
        }
    }
}

/*
impl<'a> System<'a> for Reconciliation {
    type SystemData = (
        WriteStorage<'a, Actor>,
        WriteStorage<'a, InputBuffer>,
        ReadStorage<'a, StateBuffer>,
    );

    fn run(&mut self, (mut actors, states): Self::SystemData) {
        for (actor, state) in (&mut actors, &states).join() {
            let position = state.last_recent_position();
        }
    }
}
*/
