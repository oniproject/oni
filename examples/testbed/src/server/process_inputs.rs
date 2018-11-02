use specs::prelude::*;
use std::time::Instant;
use crate::{
    components::*,
    prot::*,
    consts::*,
    util::{Circle, secs_to_duration},
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
    seq: WriteStorage<'a, LastSequence>,
    conn: WriteStorage<'a, Conn>,

    socket: WriteExpect<'a, oni::Server<oni::SimulatedSocket>>,
    node: WriteExpect<'a, NetNode>,
}

impl<'a> System<'a> for ProcessInputs {
    type SystemData = ProcessInputsData<'a>;

    fn run(&mut self, mut data: Self::SystemData) {
        oni_trace::scope![server process inputs];

        decelerator!();

        // FIXME
        for actor in (&mut data.actors).join() {
            actor.damage = false;
        }

        let now = Instant::now();

        {
            oni_trace::scope![update sock];

            let socket = &mut data.socket;
            let entities = &mut data.entities;
            let mut inputs = &mut data.inputs;
            let mut states = &mut data.states;
            let mut seq = &mut data.seq;
            let mut node = &mut data.node;
            let mut connections = &mut data.conn;
            let mut marker = &mut data.marker;

        socket.update(|conn, _user_data| {
            //let id = conn.id();
            let addr = conn.addr();
            //println!("connected[{}] {:?}", id, addr);

            let e = entities.build_entity()
                .with(InputBuffer::new(), &mut inputs)
                .with(StateBuffer::new(), &mut states)
                .with(Conn(conn), &mut connections)
                .with(LastSequence::default(), &mut seq)
                .marked(&mut marker, &mut node)
                .build();
            node.by_addr.insert(addr, e);
            debug!("register client: {} {:?}", addr, e);

            /*
            let user = unsafe { std::ffi::CStr::from_ptr(user.as_ptr() as *const _) };
            connected.push(c);
            */
        });

        }

        // Process all pending messages from clients.
        let conns = &mut data.conn;
        let node = &mut data.node;
        let actors = &mut data.actors;
        let seq = &data.seq;
        let entities = &*data.entities;
        let states = &data.states;

        for client in conns.join() {
            let addr = client.0.addr();

        while let Some(message) = client.0.recv_client() {
            match message {
            Client::Input(message) => {
                let by_addr = node.by_addr.get(&addr).cloned();
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
                        actors.get_mut(entity).and_then(|a| {
                            a.apply_input(&message);

                            let iso = a.transform();
                            if message.fire {
                                let p = &Point2::new(FIRE_LEN, 0.0);

                                let ack: u16 = message.frame_ack.into();
                                let last: u16 = seq.get(entity).unwrap().0.into();

                                let diff = f32::from(last.wrapping_sub(ack));

                                let time = secs_to_duration(diff / SERVER_UPDATE_RATE);

                                Some((a.position, iso.transform_point(p), time))
                            } else {
                                None
                            }
                        })
                    };

                    if let Some((start, end, time)) = ray {
                        let iter = (entities, &mut *actors, states)
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

                            actor.damage = circ.raycast(start, end);
                        }
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
