use serde_cbor;
use ws::{self, WebSocket, Message, Request, Response, Result, Sender};

fn map_cbor_err(e: serde_cbor::error::Error) -> ws::Error {
    ws::Error::new(ws::ErrorKind::Protocol, format!("cbor: {:?}", e))
}

use std::{
    net::ToSocketAddrs,
    fmt::Debug,
};

pub fn run<A: ToSocketAddrs + Debug>(addr: A) {
    let ws = WebSocket::new(Factory).unwrap();
    ws.listen(addr).unwrap();
}


struct Factory;

impl ws::Factory for Factory {
    type Handler = Handler;

    fn connection_made(&mut self, ws: Sender) -> Handler {
        Handler {
            ws,
            pos: (10.0, 10.0),
        }
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
                let msg: ClientMessage = serde_cbor::from_slice(&msg)
                    .map_err(map_cbor_err)?;

                match msg {
                    ClientMessage::V { x, y } => {
                        self.pos.0 += x;
                        self.pos.1 += y;
                    }
                }

                let state = [
                    Event::A {
                        id: 5,
                        pos: self.pos,
                        vel: (0.0, 0.0),
                    },
                ];

                let data = serde_cbor::to_vec(&state)
                    .map_err(map_cbor_err)?;

                self.ws.send(data)
            }

            Message::Text(msg) => self.ws.broadcast(msg),
        }
    }
}


#[derive(Serialize, Deserialize, Debug)]
pub enum Event {
    /// Actor
    A {
        id: u16,
        pos: (f32, f32),
        vel: (f32, f32),
    },
    /// Enemy
    E {
        id: u16,
        pos: (f32, f32),
        vel: (f32, f32),
    },
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

#[derive(Serialize, Deserialize, Debug)]
pub enum ClientMessage {
    V {
        x: f32,
        y: f32,
    },
}
