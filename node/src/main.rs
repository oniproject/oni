extern crate tungstenite;
extern crate fnv;
extern crate specs;
#[macro_use]
extern crate specs_derive;
extern crate shrev;
extern crate rayon;

extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate serde_cbor;

mod components;
//mod server;
mod send_world_state;
mod process_inputs;

fn main() {
    let addr = "127.0.0.1:9000";


    use specs::prelude::*;
    use std::sync::Arc;
    use rayon::ThreadPoolBuilder;

    use send_world_state::SendWorldState;
    use process_inputs::ProcessInputs;
    use components::*;

    use std::{net::TcpListener, thread::spawn};

    let mut world = World::new();

    world.register::<Position>();
    world.register::<Velocity>();
    world.register::<NetMarker>();
    world.register::<Connection>();
    world.register::<LastProcessedInput>();

    //world.add_resource();

    let pool = ThreadPoolBuilder::new().build().unwrap();
    let pool = Arc::new(pool);

    let (tx, rx) = std::sync::mpsc::sync_channel(1);
    let _ = spawn(move || {
        let server = TcpListener::bind(addr).unwrap();
        for stream in server.incoming() {
            let ws = tungstenite::accept(stream.unwrap()).unwrap();
            tx.send(ws).unwrap();
        }
    });

    let mut dispatcher = DispatcherBuilder::new()
        .with_pool(pool)
        .with(ProcessInputs, "process_inputs", &[])
        .with(SendWorldState::new(), "send_world_state", &["process_inputs"])
        .with(Closer, "connection_closer", &[])
        .build();

    let mut mark = 1;

    loop {
        dispatcher.dispatch(&world.res);

        if let Ok(ws) = rx.try_recv() {
            let entity = world.create_entity()
                .with(Connection(ws, false))
                .with(Position(4.5, 2.7))
                .with(Velocity(1.0, 1.0))
                .with(LastProcessedInput(0))
                .with(NetMarker(mark))
                .build();

            mark += 1;
        }

        world.maintain();
    }
}

use specs::prelude::*;
use components::*;

struct Closer;

impl<'a> System<'a> for Closer {
    type SystemData = (
        Entities<'a>,
        ReadStorage<'a, Connection>,
    );

    fn run(&mut self, mut data: Self::SystemData) {
        let to_remove = (&*data.0, &data.1).join()
            .filter_map(|(e, c)| if !c.1 { None } else { Some(e) });
        for entity in to_remove {
            println!("close: {:?}", entity);
            data.0.delete(entity).unwrap();
        }
    }
}
