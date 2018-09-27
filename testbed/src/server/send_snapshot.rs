use specs::{
    prelude::*,
    saveload::Marker,
};
use std::time::{Instant, Duration};
use oni::{
    simulator::Socket,
    reliable::SequenceOps,
};
use crate::{
    components::*,
    prot::*,
    prot::Endpoint,
};

// Gather the state of the world.
// In a real app, state could be filtered to avoid leaking data
// (e.g. position of invisible enemies).
pub struct SendWorldState;

#[derive(SystemData)]
pub struct SendWorldStateData<'a> {
    entities: Entities<'a>,
    socket: ReadExpect<'a, Socket>,
    mark: ReadStorage<'a, NetMarker>,
    actors: WriteStorage<'a, Actor>,
    states: WriteStorage<'a, StateBuffer>,
    lpi: ReadStorage<'a, InputBuffer>,
    conn: WriteStorage<'a, Conn>,
}

impl<'a> System<'a> for SendWorldState {
    type SystemData = SendWorldStateData<'a>;

    fn run(&mut self, mut data: Self::SystemData) {
        oni::trace::scope![server send world state];

        decelerator!();

        let now = Instant::now();

        for (a, buf) in (&data.actors, &mut data.states).join() {
            buf.drop_older(now - Duration::from_secs(1));
            buf.push_state(now, &EntityState::new(0, a.position, a.rotation, a.damage, a.fire));
        }

        for (e, lpi, conn) in (&data.mark, &data.lpi, &mut data.conn).join() {
            let states: Vec<_> = (&data.mark, &data.actors)
                .join()
                // TODO: filter
                .map(|(e, a)| EntityState::new(
                    e.id() as u8,
                    a.position,
                    a.rotation,
                    a.damage,
                    a.fire,
                ))
                .collect();

            data.socket.send_server(Server::Snapshot {
                me_id: e.id() as u8,
                frame_seq: conn.last_sequence.fetch_next(),
                states,
                ack: lpi.generate_ack(),
            }, conn.addr);
        }
    }
}
