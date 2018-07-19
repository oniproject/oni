#[macro_use] extern crate log;
extern crate env_logger;

extern crate mio;

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

mod components;
mod send_world_state;
mod process_inputs;

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

    let addr = "127.0.0.1:9000";


    use specs::prelude::*;
    use std::sync::Arc;
    use rayon::ThreadPoolBuilder;

    use send_world_state::SendWorldState;
    use process_inputs::ProcessInputs;
    use components::*;

    let mut world = World::new();

    world.register::<Position>();
    world.register::<Velocity>();
    world.register::<NetMarker>();
    world.register::<Connection<TcpStream>>();
    world.register::<LastProcessedInput>();

    //world.add_resource();

    let pool = ThreadPoolBuilder::new().build().unwrap();
    let pool = Arc::new(pool);
    let mut dispatcher = DispatcherBuilder::new()
        .with_pool(pool)
        .with(ProcessInputs, "process_inputs", &[])
        .with(SendWorldState::new(), "send_world_state", &["process_inputs"])
        .with(ConnectionManager::new(addr), "connection_manager", &[])
        .build();

    let e = EventLoop::new(Duration::from_millis(10));
    for _ in e {
        dispatcher.dispatch(&world.res);
        world.maintain();
    }
}

use tungstenite::WebSocket;
use specs::prelude::*;

use std::{
    time::{Instant, Duration},
    thread::{sleep, spawn},
    sync::{
        atomic::{AtomicUsize, Ordering},
        mpsc::{sync_channel, Receiver},
    },
    net::{TcpListener, TcpStream, ToSocketAddrs},
};

use components::*;


struct ConnectionManager {
    rx: Receiver<WebSocket<TcpStream>>,
    mark: AtomicUsize,
}

impl ConnectionManager {
    fn new<A: ToSocketAddrs>(addr: A) -> Self {
        let (tx, rx) = sync_channel(1);
        let server = TcpListener::bind(addr).unwrap();
        let _ = spawn(move || {
            for stream in server.incoming() {
                let ws = tungstenite::accept(stream.unwrap()).unwrap();
                tx.send(ws).unwrap();
            }
        });
        Self { rx, mark: AtomicUsize::new(1) }
    }
}

#[derive(SystemData)]
struct ConnData<'a> {
    e: Entities<'a>,
    conn: WriteStorage<'a, Connection<TcpStream>>,
    pos: WriteStorage<'a, Position>,
    vel: WriteStorage<'a, Velocity>,
    lpi: WriteStorage<'a, LastProcessedInput>,
    mark: WriteStorage<'a, NetMarker>,
}

impl<'a> System<'a> for ConnectionManager {
    type SystemData = ConnData<'a>;

    fn run(&mut self, mut data: Self::SystemData) {
        if let Ok(ws) = self.rx.try_recv() {
            let mark = self.mark.fetch_add(1, Ordering::SeqCst);
            let entity = data.e.build_entity()
                .with(Connection::new(ws), &mut data.conn)
                .with(Position(4.5, 2.7), &mut data.pos)
                .with(Velocity(1.0, 1.0), &mut data.vel)
                .with(LastProcessedInput(0), &mut data.lpi)
                .with(NetMarker(mark), &mut data.mark)
                .build();
            debug!("create: {:?}", entity);
        }

        let to_remove = (&*data.e, &data.conn).join()
            .filter_map(|(e, c)| if !c.err { None } else { Some(e) });
        for entity in to_remove {
            debug!("delete: {:?}", entity);
            data.e.delete(entity).unwrap();
        }
    }
}

struct EventLoop {
    quit: bool,
    dt_update: Duration,
    last_update: Instant,
}

impl EventLoop {
    fn new(dt_update: Duration) -> Self {
        Self {
            dt_update,
            quit: false,
            last_update: Instant::now(),
        }
    }

    fn quit(&mut self) {
        self.quit = true;
    }
}

impl Iterator for EventLoop {
    type Item = ();
    fn next(&mut self) -> Option<()> {
        let current_time = Instant::now();
        let next_time = self.last_update + self.dt_update;
        if next_time > current_time {
            sleep(next_time - current_time);
            self.last_update += self.dt_update;
        }

        if self.quit {
            None
        } else {
            Some(())
        }
    }
}
