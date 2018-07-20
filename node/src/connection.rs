use tungstenite::{self, WebSocket};
use specs::prelude::*;
use std::{
    io::{Read, Write},
    collections::VecDeque,
};

pub trait Stream: Read + Write + Send + Sync + 'static {}

impl<T> Stream for T where T: Read + Write + Send + Sync + 'static {}

#[derive(Component)]
#[storage(VecStorage)]
pub struct Connection<T: Stream> {
    pub ws: WebSocket<T>,
    pub err: bool,
    pub unprocessed: VecDeque<Input>,
    pub last_processed_input: u16,
}

impl<T: Stream> Connection<T> {
    pub fn new(ws: WebSocket<T>) -> Self {
        Self {
            ws,
            err: false,
            unprocessed: VecDeque::new(),
            last_processed_input: 0,
        }
    }

    fn log_err<R>(&mut self, r: tungstenite::Result<R>) {
        if let Err(e) = r {
            self.err = true;
            error!("log_err: {:?}", e);
        }
    }

    pub fn send(&mut self, data: &[u8]) {
        let e = self.ws.write_message(data.into());
        self.log_err(e);
    }

    pub fn receive(&mut self) -> Option<Input> {
        self.unprocessed.pop_front()
    }
}

impl<T: Stream> Iterator for Connection<T> {
    type Item = Input;
    fn next(&mut self) -> Option<Self::Item> {
        self.receive()
    }
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
