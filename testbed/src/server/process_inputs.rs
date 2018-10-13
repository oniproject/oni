use specs::prelude::*;
use std::time::Instant;
use oni::simulator::Socket;
use crate::{
    components::*,
    prot::*,
    prot::Endpoint,
    consts::*,
    util::{Segment, Circle, secs_to_duration},
};

// Check whether self input seems to be valid (e.g. "make sense" according
// to the physical rules of the World)
fn validate_input(input: &InputSample) -> bool {
    input.press_delta.abs() <= 1.0 / 40.0 * 1000.0
}

pub struct ProcessInputs;

#[derive(SystemData)]
pub struct ProcessInputsData<'a> {
    entities: Entities<'a>,

    marker: WriteStorage<'a, NetMarker>,
    actors: WriteStorage<'a, Actor>,
    inputs: WriteStorage<'a, InputBuffer>,
    states: WriteStorage<'a, StateBuffer>,
    conn: WriteStorage<'a, Conn>,

    socket: ReadExpect<'a, Socket>,
    node: WriteExpect<'a, NetNode>,
}

impl<'a> System<'a> for ProcessInputs {
    type SystemData = ProcessInputsData<'a>;

    fn run(&mut self, mut data: Self::SystemData) {
        oni::trace::scope![server process inputs];

        decelerator!();

        // FIXME
        for actor in (&mut data.actors).join() {
            actor.damage = false;
        }

        let now = Instant::now();

        // Process all pending messages from clients.
        while let Some((message, addr)) = data.socket.recv_client() {
            match message {
            Client::Start => {
                let e = data.entities.build_entity()
                    .with(InputBuffer::new(), &mut data.inputs)
                    .with(StateBuffer::new(), &mut data.states)
                    .with(Conn::new(addr), &mut data.conn)
                    .marked(&mut data.marker, &mut data.node)
                    .build();
                data.node.by_addr.insert(addr, e);
                debug!("register client: {} {:?}", addr, e);
            }
            Client::Input(message) => {
                let by_addr = data.node.by_addr.get(&addr).cloned();
                let entity = if let Some(e) = by_addr {
                    e
                } else {
                    println!("server just ignore message");
                    continue;
                };

                let buf = data.inputs.get_mut(entity).unwrap();

                for message in message {
                // We just ignore inputs that don't look valid;
                // self is what prevents clients from cheating.
                if buf.insert(message.sequence) && validate_input(&message) {
                    use alga::linear::Transformation;
                    use nalgebra::Point2;

                    // Update the state of the entity, based on its input.
                    let ray = {
                        let conn = data.conn.get(entity).unwrap();

                        data.actors.get_mut(entity).and_then(|a| {
                            a.apply_input(&message);

                            let iso = a.transform();
                            if message.fire {
                                let p = &Point2::new(FIRE_LEN, 0.0);

                                let ack: u16 = message.frame_ack.into();
                                let last: u16 = conn.last_sequence.into();

                                let diff = last.wrapping_sub(ack) as f32;

                                let time = secs_to_duration(diff / SERVER_UPDATE_RATE);

                                Some((a.position, iso.transform_point(p), time))
                            } else {
                                None
                            }
                        })
                    };

                    if let Some((start, end, time)) = ray {
                        let iter = (&*data.entities, &mut data.actors, &data.states)
                            .join()
                            .filter(|(e, _, _)| *e != entity);

                        for (_, actor, state) in iter {
                            let center = state
                                .interpolate_linear(now - time - RENDER_TIME)
                                .unwrap_or(actor.position);

                            let circ = Circle {
                                center,
                                radius: FIRE_RADIUS,
                            };

                            actor.damage = circ.raycast(Segment {
                                start, end,
                            });
                        }
                    }
                }
                }
            }
            }
        }
    }
}

/*
pub struct ShotSystem;

impl<'a> System<'a> for ShotSystem {
    type SystemData = (
        WriteStorage<'a, Actor>,
        ReadStorage<'a, InputBuffer>,
        ReadStorage<'a, StateBuffer>,
    );

    fn run(&mut self, (mut actors, inputs, states): Self::SystemData) {
    }
}
*/
