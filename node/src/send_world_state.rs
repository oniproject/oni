use serde_cbor;
use specs::prelude::*;

use components::*;

/// Send the world state to all the connected clients.
pub struct SendWorldState {
    temp_buf: Vec<u8>,
}

impl SendWorldState {
    pub fn new() -> Self {
        Self {
            temp_buf: Vec::new(),
        }
    }
}

impl<'a> System<'a> for SendWorldState {
    type SystemData = (
        ReadStorage<'a, NetMarker>,
        ReadStorage<'a, Position>,
        ReadStorage<'a, Velocity>,
        ReadStorage<'a, LastProcessedInput>,
        WriteStorage<'a, Connection>,
    );

    fn run(&mut self, (ref mark, ref pos, ref vel, ref input, ref mut ws): Self::SystemData) {
        // Gather the state of the world.
        // In a real app, state could be filtered to avoid leaking data
        // (e.g. position of invisible enemies).
        let states: Vec<_> = (mark, pos, vel).join()
            .map(|(e, p, v)| (e.clone(), p.clone(), v.clone())).collect();

        // Broadcast the state to all the clients.
        for (entity, last_processed_input, client) in (mark, input, ws).join() {
            let state = ("W", entity, last_processed_input, states.clone());

            self.temp_buf.clear();
            serde_cbor::to_writer(&mut self.temp_buf, &state).unwrap();
            client.send(&self.temp_buf);
        }
    }
}
