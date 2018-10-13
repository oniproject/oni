use specs::prelude::*;
use shrev::{EventChannel, EventIterator, ReaderId};
use crossbeam::queue::SegQueue;

use std::net::{SocketAddr, UdpSocket};
use std::sync::Arc;

pub struct OniConnection {
    /// The buffer of events that have been received.
    recv_buffer: SegQueue<OniNetEvent>,
    /// The buffer of events to be sent.
    send_buffer: SegQueue<OniNetEvent>,
}

impl Component for OniConnection {
    type Storage = FlaggedStorage<Self, DenseVecStorage<Self>>;
}

impl OniConnection {
    fn new() -> Self {
        Self {
            recv_buffer: SegQueue::new(),
            send_buffer: SegQueue::new(),
        }
    }
}

struct RawEvent {
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum OniNetEvent {
    //
}

/// The state of the connection.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConnectionState {
    /// The connection is established.
    Connected,
    /// The connection is being established.
    Connecting,
    /// The connection has been dropped.
    Disconnected,
}

struct Channel<A, B> {
    tx: SegQueue<A>,
    rx: SegQueue<B>,
}

pub struct OniServerSystem {
    ch: Arc<Channel<RawEvent, (usize, SocketAddr)>>,
    //removed_reader_id: ReaderId<RemovedFlag>,
}

impl OniServerSystem {
    /// Creates a `OniSystem` and binds the Socket on the ip and port added in parameters.
    pub fn new(addr: SocketAddr) -> std::io::Result<Self> {
        let socket = Arc::new(UdpSocket::bind(addr)?);
        socket.set_nonblocking(true)?;

        let ch = Arc::new(Channel {
            tx: SegQueue::new(),
            rx: SegQueue::new(),
        });

        let chan = ch.clone();

        std::thread::spawn(move || {
            while let Some(packet) = chan.tx.try_pop() {
                //
            }

            let mut buffer = [0u8; crate::protocol::MTU];
            while let Ok((len, addr)) = socket.recv_from(&mut buffer) {
                // TODO
                chan.rx.push((len, addr));
            }
        });

        Ok(Self { ch })
    }
}

impl<'a> System<'a> for OniServerSystem {
    type SystemData = (WriteStorage<'a, OniConnection>);

    fn setup(&mut self, res: &mut Resources) {
        Self::SystemData::setup(res);
    }

    fn run(&mut self, mut connections: Self::SystemData) {
        //for mut connection in (&mut connections).join() {

        /*
        for mut connection in (&mut net_connections).join() {
            let target = net_connection.target.clone();

            if net_connection.state == ConnectionState::Connected
                || net_connection.state == ConnectionState::Connecting
            {
                self.tx
                    .send(InternalSocketEvent::SendEvents {
                        target,
                        events: net_connection.send_buffer_early_read().cloned().collect(),
                    }).unwrap();
            } else if net_connection.state == ConnectionState::Disconnected {
                self.tx.send(InternalSocketEvent::Stop).unwrap();
            }
        }

        for raw_event in self.rx.try_iter() {
            let mut matched = false;
            // Get the NetConnection from the source
            for mut net_connection in (&mut net_connections).join() {
                // We found the origin
                if net_connection.target == raw_event.source {
                    matched = true;
                    // Get the event
                    let net_event = deserialize_event::<E>(raw_event.data.as_slice());
                    match net_event {
                        Ok(ev) => {
                            net_connection.receive_buffer.single_write(ev);
                        }
                        Err(e) => error!(
                            "Failed to deserialize an incoming network event: {} From source: {:?}",
                            e, raw_event.source
                        ),
                    }
                }
                if !matched {
                    println!("Received packet from unknown source");
                }
            }
        }
        */
    }
}
