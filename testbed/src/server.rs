use specs::{
    prelude::*,
    saveload::Marker,
};
use std::time::{Instant, Duration};
use oni::simulator::Socket;
use crate::{
    components::*,
    prot::*,
    prot::Endpoint,
    consts::*,
    ui::Demo,
};

pub fn new_server(network: Socket) -> Demo {
    let mut world = World::new();
    world.register::<Conn>();
    world.register::<Actor>();
    world.register::<NetMarker>();
    world.register::<InputBuffer>();
    world.register::<StateBuffer>();

    world.add_resource(network);
    world.add_resource(NetNode::new(0..2));

    Demo::new(SERVER_UPDATE_RATE, world, DispatcherBuilder::new()
        .with(ProcessInputs, "ProcessInputs", &[])
        .with(SendWorldState, "SendWorldState", &["ProcessInputs"]))
}

pub struct ProcessInputs;

unsafe impl Send for ProcessInputs {}
unsafe impl Sync for ProcessInputs {}

impl<'a> System<'a> for ProcessInputs {
    type SystemData = (
        Entities<'a>,
        ReadExpect<'a, Socket>,
        WriteStorage<'a, Actor>,
        WriteStorage<'a, InputBuffer>,
        ReadStorage<'a, StateBuffer>,
        ReadExpect<'a, NetNode>,
    );

    fn run(&mut self, (entities, socket, mut actors, mut lpi, states, node): Self::SystemData) {
        for actor in (&mut actors).join() {
            actor.damage = false;
        }

        let now = Instant::now();

        // Process all pending messages from clients.
        while let Some((message, addr)) = socket.recv_input() {
            let entity = node.by_addr.get(&addr).cloned().unwrap();
            let buf = lpi.get_mut(entity).unwrap();
            // We just ignore inputs that don't look valid;
            // self is what prevents clients from cheating.
            if buf.insert(message.sequence) && validate_input(&message) {

                use alga::linear::Transformation;
                use nalgebra::Point2;

                // Update the state of the entity, based on its input.
                let ray = {
                    let a = actors.get_mut(entity).unwrap();
                    a.apply_input(&message);

                    let iso = a.transform();
                    if message.fire {
                        let p = &Point2::new(FIRE_LEN, 0.0);
                        Some((a.position, iso.transform_point(p)))
                    } else {
                        None
                    }
                };

                if let Some((start, end)) = ray {
                    let iter = (&*entities, &mut actors, &states)
                        .join()
                        .filter(|(e, _, _)| *e != entity);

                    for (_, actor, state) in iter {
                        let center = state
                            .interpolate_linear(now - Duration::from_millis(400))
                            //.unwrap();
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

use nalgebra::{Point2, dot};

struct Segment {
    start: Point2<f32>,
    end: Point2<f32>,
}

struct Circle {
    center: Point2<f32>,
    radius: f32,
}

impl Circle {
    fn raycast(&self, ray: Segment) -> bool {
        let d = ray.end - ray.start;
        let f = ray.start - self.center;

        let a = dot(&d, &d);
        let b = 2.0 * dot(&f, &d);
        let c = dot(&f, &f) - self.radius * self.radius;

        let discriminant = b * b - 4.0 * a * c;
        if discriminant < 0.0 {
            return false;
        }

        let discriminant = discriminant.sqrt();

        let t1 = (-b - discriminant) / (2.0 * a);
        let t2 = (-b + discriminant) / (2.0 * a);

        t1 >= 0.0 && t1 <= 1.0 || t2 >= 0.0 && t2 <= 1.0
    }
}

// Check whether self input seems to be valid (e.g. "make sense" according
// to the physical rules of the World)
fn validate_input(input: &Input) -> bool {
    input.press_time.abs() <= 1.0 / 40.0 * 1000.0
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
    addr: WriteStorage<'a, Conn>,
}

impl<'a> System<'a> for SendWorldState {
    type SystemData = SendWorldStateData<'a>;

    fn run(&mut self, mut data: Self::SystemData) {
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

        // Broadcast the state to all the clients.
        for (lpi, addr) in (&data.lpi, &mut data.addr).join() {
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
                states,
                ack: lpi.generate_ack(),
            }, addr.0);
        }
    }
}
