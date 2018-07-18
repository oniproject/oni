use specs::prelude::*;

use shrev::{EventChannel, ReaderId};

use serde_cbor;
use ws::{self, WebSocket, Message, Request, Response, Result, Sender};

use fnv::FnvHashMap;

fn map_cbor_err(e: serde_cbor::error::Error) -> ws::Error {
    ws::Error::new(ws::ErrorKind::Protocol, format!("cbor: {:?}", e))
}

use std::{
    net::ToSocketAddrs,
    fmt::Debug,
};

pub fn run<A: ToSocketAddrs + Debug>(addr: A) {
    let ws = WebSocket::new(Factory::new()).unwrap();
    ws.listen(addr).unwrap();
}

struct Factory {
    mapping: FnvHashMap<ws::util::Token, Sender>,

    made: ReaderId<Sender>,
    lost: ReaderId<Sender>,
    made_ch: EventChannel<Sender>,
    lost_ch: EventChannel<Sender>,
}

impl Factory {
    fn new() -> Self {
        let mut made_ch = EventChannel::new();
        let mut lost_ch = EventChannel::new();
        let made = made_ch.register_reader();
        let lost = lost_ch.register_reader();
        Self {
            mapping: FnvHashMap::default(),
            made_ch,
            lost_ch,
            made,
            lost,
        }
    }
}

#[derive(Component)]
#[storage(DenseVecStorage)]
struct WS(Sender);

impl<'a> System<'a> for Factory {
    type SystemData = (
        WriteStorage<'a, WS>,
        Entities<'a>,
    );

    fn run(&mut self, (mut sockets, mut entities): Self::SystemData) {
        for ws in self.made_ch.read(&mut self.made) {
            let entity = entities.create();
            sockets.insert(entity, WS(ws.clone())).unwrap();
        }

        for ws in self.lost_ch.read(&mut self.lost) {
            // TODO
        }
    }
}

impl ws::Factory for Factory {
    type Handler = Handler;

    fn connection_made(&mut self, ws: Sender) -> Self::Handler {
        self.mapping.insert(ws.token(), ws.clone());
        self.made_ch.single_write(ws.clone());

        Handler {
            ws,
            pos: (10.0, 10.0),
        }
    }

    fn connection_lost(&mut self, handler: Self::Handler) {
        let Handler { ws, .. } = handler;
        self.mapping.remove(&ws.token()).unwrap();
        self.lost_ch.single_write(ws);
    }
}

struct Handler {
    ws: Sender,
    pos: (f32, f32),
}

impl ws::Handler for Handler {
    fn on_request(&mut self, req: &Request) -> Result<(Response)> {
        match req.resource() {
            "/ws" => Response::from_request(req),
            //"/" => Ok(Response::new(200, "OK", INDEX_HTML.to_vec())),
            _ => Ok(Response::new(404, "Not Found", b"404 - Not Found".to_vec())),
        }
    }

    fn on_message(&mut self, msg: Message) -> Result<()> {
        match msg {
            Message::Binary(msg) => {
                let msg: UserInput = serde_cbor::from_slice(&msg)
                    .map_err(map_cbor_err)?;

                match msg {
                    UserInput::V { x, y } => {
                        self.pos.0 += x;
                        self.pos.1 += y;
                    }
                }

                let state = [
                    (5, self.pos, (0.0, 0.0)),
                ];

                let data = serde_cbor::to_vec(&state)
                    .map_err(map_cbor_err)?;

                self.ws.send(data)
            }

            Message::Text(msg) => self.ws.broadcast(msg),
        }
    }
}


/*
{
    let state = SyncState::Actor {
        position: (10.0, 10.0),
        velocity: (0.0, 0.0),
    };

    let data = serde_cbor::to_vec(&state).unwrap();
    println!("to_vec: {:?}", data);

    let data: SyncState = serde_cbor::from_slice(&data).unwrap();
    println!("from_slice: {:?}", data);
}
*/

struct State {
    processed: Vec<UserInput>,
    unprocessed: Vec<UserInput>,
}

#[derive(Serialize, Deserialize, Debug)]
pub enum UserInput {
    V {
        x: f32,
        y: f32,
    },
}

#[derive(Component, Default)]
#[storage(VecStorage)]
struct Pos(f32, f32);

#[derive(Component, Default)]
#[storage(VecStorage)]
struct Vel(f32, f32);

struct InputBuffer {
    buf: Vec<UserInput>,
}

struct Input;

impl<'a> System<'a> for Input {
    type SystemData = (
        Write<'a, Vel>,
        WriteExpect<'a, InputBuffer>,
    );

    fn run(&mut self, (ref mut sockets, ref mut entities): Self::SystemData) {
    }
}

struct Movement;

impl<'a> System<'a> for Movement {
    type SystemData = (
        WriteStorage<'a, Pos>,
        ReadStorage<'a, Vel>,
    );

    fn run(&mut self, (ref mut pos, ref vel): Self::SystemData) {
        for (p, v) in (pos, vel).join() {
            p.0 += v.0;
            p.1 += v.1;
        }
    }
}
