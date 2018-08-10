use specs::prelude::*;
use serde_cbor;
use fnv::FnvHashMap;
use tungstenite::{
    Message,
    HandshakeError,
    ServerHandshake,
    handshake::server::NoCallback,
};
use mio::*;
use mio::net::{TcpListener, TcpStream};

use std::{
    sync::atomic::{AtomicUsize, Ordering},
    net::SocketAddr,
};

use crate::components::*;
use crate::connection::*;
use crate::net_marker::*;

const SERVER: Token = Token(0);

#[derive(SystemData)]
pub struct ConnData<'a> {
    e: Entities<'a>,
    conn: WriteStorage<'a, Connection<TcpStream>>,
    pos: WriteStorage<'a, Position>,
    vel: WriteStorage<'a, Velocity>,
    mark: WriteStorage<'a, NetMarker>,
    mark_alloc: WriteExpect<'a, NetNode>,
}

pub struct Network {
    mark: AtomicUsize,
    server: ::mio::net::TcpListener,
    poll: Poll,
    events: Events,
    mapping: FnvHashMap<Token, Entity>,
}

impl Network {
    pub fn new(addr: SocketAddr) -> Self {
        let server = TcpListener::bind(&addr).unwrap();
        let poll = Poll::new().unwrap();
        poll.register(&server, SERVER, Ready::readable(),
              PollOpt::edge()).unwrap();
        Self {
            mark: AtomicUsize::new(1),
            server,
            poll,
            events: Events::with_capacity(1024),
            mapping: FnvHashMap::default(),
        }
    }
}

impl<'a> System<'a> for Network {
    type SystemData = ConnData<'a>;

    fn run(&mut self, mut data: Self::SystemData) {
        self.poll.poll(&mut self.events, None).unwrap();

        for event in self.events.iter() {
            let token = event.token();
            if token == SERVER {
                let (stream, _addr) = self.server.accept().unwrap();
                stream.set_nodelay(true).unwrap();

                let mark = self.mark.fetch_add(1, Ordering::SeqCst);
                let token = Token(mark);

                self.poll.register(&stream, token, Ready::readable(),
                        PollOpt::edge()).unwrap();

                // FIXME: may fuck server
                let mut hs = ServerHandshake::start(stream, NoCallback, None);
                let stream = loop {
                    match hs.handshake() {
                        Ok(s) => break s,
                        Err(HandshakeError::Interrupted(chain)) => hs = chain,
                        Err(HandshakeError::Failure(err)) => panic!("e: {:?}", err),
                    }
                };

                let entity = data.e.build_entity()
                    .with(Connection::new(stream), &mut data.conn)
                    .with(Position(4.5, 2.7), &mut data.pos)
                    .with(Velocity(1.0, 1.0), &mut data.vel)
                    .marked(&mut data.mark, &mut data.mark_alloc)
                    .build();

                debug!("insert: {:?} {:?} {:?}", token, entity, _addr);
                self.mapping.insert(token, entity);
            } else {
                let entity = self.mapping[&token];
                let conn = data.conn.get_mut(entity).unwrap();
                match conn.ws.read_message() {
                    Ok(Message::Binary(data)) => {
                        let m = serde_cbor::from_slice(&data).unwrap();
                        conn.unprocessed.push_back(m);
                    }
                    Ok(Message::Text(msg)) => {
                        println!("chat: {}", msg);
                    }
                    Err(_) => conn.err = true,
                    _ => (),
                }
            }
        }


        let to_remove = (&*data.e, &data.conn).join()
            .filter_map(|(e, c)| if !c.err { None } else { Some(e) });
        for entity in to_remove {
            debug!("delete: {:?}", entity);
            data.e.delete(entity).unwrap();
        }
    }
}
