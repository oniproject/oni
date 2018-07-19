use serde_cbor;
use tungstenite::{self, WebSocket, Message};
use specs::prelude::*;
use std::io::{Read, Write};

#[derive(Component, Serialize, Default, Clone)]
#[storage(VecStorage)]
pub struct NetMarker(pub usize);

#[derive(Component, Serialize, Default, Clone)]
#[storage(VecStorage)]
pub struct LastProcessedInput(pub u16);

#[derive(Component, Serialize, Default, Clone)]
#[storage(VecStorage)]
pub struct Position(pub f32, pub f32);

#[derive(Component, Serialize, Default, Clone)]
#[storage(VecStorage)]
pub struct Velocity(pub f32, pub f32);

#[derive(Component)]
#[storage(VecStorage)]
pub struct Connection<T: Send + Sync + 'static> {
    pub ws: WebSocket<T>,
    pub err: bool,
}

impl<T: Read + Write + Send + Sync + 'static> Connection<T> {
    pub fn new(ws: WebSocket<T>) -> Self {
        Self {
            ws,
            err: false,
        }
    }

    fn log_err<R>(&mut self, r: tungstenite::Result<R>) {
        if let Err(e) = r {
            self.err = true;
            println!("error: {:?}", e);
        }
    }

    pub fn send(&mut self, data: &[u8]) {
        let e = self.ws.write_message(data.into());
        self.log_err(e);
    }

    pub fn send_chat(&mut self, data: &str) {
        let e = self.ws.write_message(data.into());
        self.log_err(e);
    }

    pub fn receive(&mut self) -> Option<Msg> {
        self.ws.read_message().ok()
            .and_then(|m| match m {
                Message::Binary(data) => {
                    let m = serde_cbor::from_slice(&data).unwrap();
                    Some(Msg::Input(m))
                }
                Message::Text(txt) => Some(Msg::Chat(txt)),
                _ => None
            })
    }
}

impl<T: Read + Write + Send + Sync + 'static> Iterator for Connection<T> {
    type Item = Msg;
    fn next(&mut self) -> Option<Self::Item> {
        self.receive()
    }
}

pub enum Msg {
    Chat(String),
    Input(Input),
}

#[derive(Deserialize, Serialize)]
pub struct Input {
    pub press_time: f32,
    pub velocity: (f32, f32),
    pub input_sequence_number: u16,
}

impl Input {
    pub fn validate(&self) -> bool {
        true
        //self.press_time.abs() <= 1.0 / 40.0
    }
}
