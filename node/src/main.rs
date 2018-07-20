#![feature(drain_filter)]

#[macro_use] extern crate log;
extern crate env_logger;

extern crate mio;
extern crate rand;

extern crate tungstenite;
extern crate fnv;
extern crate specs;
#[macro_use]
extern crate specs_derive;
//extern crate shrev;
extern crate rayon;

extern crate shred;
#[macro_use]
extern crate shred_derive;

extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate serde_cbor;

mod seq;
mod simulator;

mod components;
mod send_world_state;
mod process_inputs;
mod event_loop;
mod connection;
mod network;
mod net_marker;

fn main() {
    env_logger::init();

    {
        use mio::{Events, Poll};
        use std::time::Duration;

        let mut events = Events::with_capacity(1024);
        let poll = Poll::new().unwrap();

        // Register handles with `poll`

        poll.poll(&mut events, Some(Duration::from_millis(100))).unwrap();

        for event in events.iter() {
            println!("event={:?}", event);
        }

        sleep(Duration::from_millis(1500));

        for event in events.iter() {
            println!("event={:?}", event);
        }
    }

    println!("start");

    let addr = "127.0.0.1:9000".parse().unwrap();

    use specs::prelude::*;
    use std::time::Duration;
    use std::sync::Arc;
    use mio::net::TcpStream;
    use std::thread::sleep;
    use rayon::ThreadPoolBuilder;

    use send_world_state::SendWorldState;
    use process_inputs::ProcessInputs;
    use components::*;

    let mut world = World::new();

    world.add_resource(
        net_marker::NetNode {
            range: 0..100,
            mapping: fnv::FnvHashMap::default(),
        },
    );

    world.register::<Position>();
    world.register::<Velocity>();
    world.register::<net_marker::NetMarker>();
    world.register::<connection::Connection<TcpStream>>();

    //world.add_resource();

    let pool = ThreadPoolBuilder::new().build().unwrap();
    let pool = Arc::new(pool);
    let mut dispatcher = DispatcherBuilder::new()
        .with_pool(pool)
        .with(ProcessInputs, "process_inputs", &[])
        .with(SendWorldState::new(), "send_world_state", &["process_inputs"])
        .with(network::Network::new(addr), "connection_manager", &[])
        .build();

    let e = event_loop::EventLoop::new(Duration::from_millis(33));
    for _ in e {
        dispatcher.dispatch(&world.res);
        world.maintain();
    }
}
