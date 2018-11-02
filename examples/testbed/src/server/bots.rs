use std::f32::consts::{PI, FRAC_PI_2};

use nalgebra::{
    UnitComplex,
    Point2,
    wrap,
};

use specs::prelude::*;
use std::time::Instant;
use std::net::SocketAddr;
use crate::{
    components::*,
    prot::*,
    consts::*,
    input::*,
};

/*
struct Bot {
    position: Point2<f32>,
    rotation: f32,

    direction: f32,
    turn_speed: f32,
    speed: f32,
}
*/

#[derive(Component)]
#[storage(DenseVecStorage)]
pub struct StupidBot {
    position: Point2<f32>,
    direction: f32,
    turn_speed: f32,
}

impl StupidBot {
    pub fn new() -> Self {
        let x: f32 = rand::random();
        let y: f32 = rand::random();
        let r: f32 = rand::random();
        let s: f32 = rand::random();
        Self {
            position: Point2::new(x - 0.5, y - 0.5),
            direction: r * PI * 2.0,
            turn_speed: s - 0.8,
        }
    }
}

pub struct Stupid;

#[derive(SystemData)]
pub struct StupidData<'a> {
    actors: WriteStorage<'a, Actor>,
    stupid: WriteStorage<'a, StupidBot>,
}

impl<'a> System<'a> for Stupid {
    type SystemData = StupidData<'a>;

    fn run(&mut self, mut data: Self::SystemData) {
        let speed = 1.0 / 10.0;
        let w2 = 12.0;
        let h2 = 2.5;
        for (actor, bot) in (&mut data.actors, &mut data.stupid).join() {
            bot.direction += bot.turn_speed * 0.1;

            let (s, c) = bot.direction.sin_cos();

            bot.position.x += s * speed;
            bot.position.y += c * speed;

            // wrap the bots around as the crawl
            bot.position.x = wrap(bot.position.x, -w2, w2);
            bot.position.y = wrap(bot.position.y, -h2, h2);

            let angle = -bot.direction + FRAC_PI_2;
            actor.rotation = UnitComplex::from_angle(angle);
            //actor.rotation = bot.direction;
            actor.position = bot.position;
        }
    }
}

pub struct DDOSer {
    position: Point2<f32>,
    direction: f32,
    turn_speed: f32,

    socket: oni::Client<oni::SimulatedSocket>,

    input_sequence: oni_reliable::Sequence<u8>,
    input_sender: InputSender,
    last_processed: Instant,

    last_frame: Option<oni_reliable::Sequence<u16>>,
}

impl DDOSer {
    pub fn new(id: u64, server: SocketAddr) -> Self {
        use std::io::Write;

        let mut server_list = oni::ServerList::new();
        server_list.push(server).unwrap();

        let data = server_list.serialize().unwrap();
        let mut user = [0u8; oni::token::USER];
        (&mut user[..]).write_all(b"some user data\0").unwrap();

        let mut socket = oni::Client::simulated(PROTOCOL_ID, &oni::token::PublicToken::generate(
            data, user,
            CONNECT_TOKEN_EXPIRY,
            CONNECT_TOKEN_TIMEOUT,
            id,
            PROTOCOL_ID,
            &PRIVATE_KEY,
        ));

        use oni_reliable::Sequence;

        socket.connect(server).unwrap();

        let r: f32 = rand::random();
        let s: f32 = rand::random();
        Self {
            socket,

            position: Point2::new(0.0, 0.0),

            direction: r * PI * 2.0,
            turn_speed: s - 0.8,

            input_sender: InputSender::new(),
            last_processed: Instant::now(),

            input_sequence: Sequence::default(),
            last_frame: None,
        }
    }

    pub fn update(&mut self) {
        use crate::util::*;
        use oni_reliable::SequenceOps;

        self.socket.update();
        if !self.socket.is_connected() {
            return;
        }

        while let Some(message) = self.socket.recv_server() {
            match message {
                Server::Snapshot { frame_seq, states, .. } => {
                    self.last_frame = Some(frame_seq);
                    for m in &states {
                        if m.entity_id() == 0 {
                            self.position = m.position();
                        }
                    }
                }
            }
        }

        let frame_ack = if let Some(f) = self.last_frame {
            f
        } else {
            return;
        };

        let press_delta = {
            let now = Instant::now();
            let last = std::mem::replace(&mut self.last_processed, now);
            duration_to_secs(now - last)
        };

        //let speed = 1.0 / 10.0;
        let w2 = 12.0;
        let h2 = 2.0;

        let stick = {
            let r: f32 = rand::random();
            self.turn_speed += (0.5 - r) * 0.01;
            self.turn_speed = wrap(self.turn_speed, -PI, PI);
            self.direction += self.turn_speed * 0.1;

            if r > 0.9 {
                self.turn_speed = 0.8 - r;
            }

            let x = self.position.x;
            let y = self.position.y;
            if x.abs() >= w2 || y.abs() >= h2 {
                let m = self.position.coords.normalize();
                self.direction = UnitComplex::from_cos_sin_unchecked(-m.x, m.y).angle()  + FRAC_PI_2;
            }

            let (sin, cos) = self.direction.sin_cos();
            [sin, cos]
        };

        let input = InputSample {
            frame_ack,
            press_delta,
            stick,
            rotation: -self.direction + FRAC_PI_2,
            sequence: self.input_sequence.fetch_next(),
            fire: false,
        };

        let inputs: arrayvec::ArrayVec<_> = self.input_sender.send(input).collect();
        if !inputs.is_empty() {
            self.socket.send_client(Client::Input(inputs));
        }
    }
}
