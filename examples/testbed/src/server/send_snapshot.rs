use specs::{
    prelude::*,
    saveload::Marker,
};
use std::time::{Instant, Duration};
use oni::SimulatedSocket as Socket;
use oni_reliable::SequenceOps;
use crate::{
    components::*,
    prot::*,
};

// Gather the state of the world.
// In a real app, state could be filtered to avoid leaking data
// (e.g. position of invisible enemies).
pub struct SendWorldState;

#[derive(SystemData)]
pub struct SendWorldStateData<'a> {
    mark: ReadStorage<'a, NetMarker>,
    actors: WriteStorage<'a, Actor>,
    states: WriteStorage<'a, StateBuffer>,
    lpi: ReadStorage<'a, InputBuffer>,
    conn: WriteStorage<'a, Conn>,
    seq: WriteStorage<'a, LastSequence>,
}

impl<'a> System<'a> for SendWorldState {
    type SystemData = SendWorldStateData<'a>;

    fn run(&mut self, mut data: Self::SystemData) {
        oni_trace::scope![server send world state];

        decelerator!();

        let now = Instant::now();

        for (a, buf) in (&data.actors, &mut data.states).join() {
            buf.drop_older(now - Duration::from_secs(1));
            buf.push_state(now, &EntityState::new(0, a.position, a.rotation, a.damage, a.fire));
        }

        for (e, lpi, conn, seq) in (&data.mark, &data.lpi, &mut data.conn, &mut data.seq).join() {
            let me = e.id() as u16;
            let states: Vec<_> = (&data.mark, &data.actors)
                .join()
                // TODO: filter?
                .map(|(e, a)| {
                    let id = e.id() as u16;
                    let id = if id == me { 0 } else { id };
                    EntityState::new(id, a.position, a.rotation, a.damage, a.fire)
                })
                .collect();

            let current_frame = seq.0.fetch_next();
            conn.0.send_server(Server::Snapshot {
                frame_seq: current_frame,
                states,
                ack: lpi.generate_ack(),
            });
        }
    }
}
