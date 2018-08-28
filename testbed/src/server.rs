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
    consts::*,
    ui::Demo,
    util::{Segment, Circle, secs_to_duration},
};

pub fn new_server(pool: std::sync::Arc<rayon::ThreadPool>, network: Socket) -> Demo {
    let mut world = World::new();
    world.register::<Conn>();
    world.register::<Actor>();
    world.register::<NetMarker>();
    world.register::<InputBuffer>();
    world.register::<StateBuffer>();

    world.add_resource(network);
    world.add_resource(NetNode::new(0..2));

    Demo::new(SERVER_UPDATE_RATE, world, DispatcherBuilder::new().with_pool(pool)
        .with(ProcessInputs, "ProcessInputs", &[])
        .with(SendWorldState, "SendWorldState", &["ProcessInputs"]))
}

pub struct ProcessInputs;

#[derive(SystemData)]
pub struct ProcessInputsData<'a> {
    entities: Entities<'a>,
    actors: WriteStorage<'a, Actor>,
    input: WriteStorage<'a, InputBuffer>,
    states: ReadStorage<'a, StateBuffer>,
    conn: ReadStorage<'a, Conn>,

    socket: ReadExpect<'a, Socket>,
    node: ReadExpect<'a, NetNode>,
}

impl<'a> System<'a> for ProcessInputs {
    type SystemData = ProcessInputsData<'a>;

    fn run(&mut self, mut data: Self::SystemData) {
        oni::trace::scope![server process inputs];

        for actor in (&mut data.actors).join() {
            actor.damage = false;
        }

        let now = Instant::now();

        // Process all pending messages from clients.
        while let Some((message, addr)) = data.socket.recv_input() {
            let entity = data.node.by_addr.get(&addr).cloned().unwrap();
            let buf = data.input.get_mut(entity).unwrap();
            // We just ignore inputs that don't look valid;
            // self is what prevents clients from cheating.
            if buf.insert(message.sequence) && validate_input(&message) {

                use alga::linear::Transformation;
                use nalgebra::Point2;

                // Update the state of the entity, based on its input.
                let ray = {
                    let conn = data.conn.get(entity).unwrap();

                    let a = data.actors.get_mut(entity).unwrap();
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

// Check whether self input seems to be valid (e.g. "make sense" according
// to the physical rules of the World)
fn validate_input(input: &Input) -> bool {
    input.press_delta.abs() <= 1.0 / 40.0 * 1000.0
}

// Gather the state of the world.
// In a real app, state could be filtered to avoid leaking data
// (e.g. position of invisible enemies).
pub struct SendWorldState;

#[derive(SystemData)]
pub struct SendWorldStateData<'a> {
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

        let now = Instant::now();

        for (a, buf) in (&data.actors, &mut data.states).join() {
            buf.drop_older(now - Duration::from_secs(1));
            buf.push_state(now, &EntityState {
                entity_id: 0,
                position: a.position,
                //velocity: a.velocity,
                rotation: a.rotation.angle(),
                damage: a.damage,
                fire: a.fire,
            });
        }

        for (lpi, conn) in (&data.lpi, &mut data.conn).join() {
            let states: Vec<_> = (&data.mark, &data.actors)
                .join()
                // TODO: filter
                .map(|(e, a)| EntityState {
                    entity_id: e.id(),
                    position: a.position,
                    //velocity: a.velocity,
                    rotation: a.rotation.angle(),
                    damage: a.damage,
                    fire: a.fire,
                })
                .collect();

            data.socket.send_world(WorldState {
                frame_seq: conn.last_sequence.fetch_next(),
                states,
                ack: lpi.generate_ack(),
            }, conn.addr);
        }
    }
}
