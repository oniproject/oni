use specs::prelude::*;
use specs::saveload::MarkedBuilder;
use nalgebra::Point2;
use oni::simulator::Socket;
use crate::{
    components::*,
    consts::*,
    ui::Demo,
};

mod process_inputs;
mod send_snapshot;
mod spawner;
mod bots;

use self::process_inputs::ProcessInputs;
use self::send_snapshot::SendWorldState;
use self::spawner::Spawner;
use self::bots::{Stupid, StupidBot};

pub use self::bots::DDOSer;

/*
pub struct ServerTime {
    tick: usize,
}

impl ServerTime {
    pub fn new() -> Self {
        Self {
            tick: 0,
        }
    }
    pub fn advance(&mut self) {
        self.tick += 1;
    }
    pub fn get(&self) -> usize {
        self.tick
    }
}
*/

pub fn new_server(dispatcher: DispatcherBuilder<'static, 'static>, network: Socket) -> Demo {
    let mut world = World::new();
    world.register::<Conn>();
    world.register::<Actor>();
    world.register::<NetMarker>();
    world.register::<InputBuffer>();
    world.register::<StateBuffer>();

    world.register::<StupidBot>();

    //world.add_resource(ServerTime::new());
    world.add_resource(network);
    world.add_resource(NetNode::new(1..150));

    if false {
        for _ in 0..120 {
            let pos = Point2::origin();
            let _e = world.create_entity()
                .marked::<NetMarker>()
                .with(Actor::spawn(pos))
                .with(StateBuffer::new())
                .with(StupidBot::new())
                .build();
        }
    }

    let dispatcher = dispatcher
        .with(Stupid, "Stupid bots with stupid AI", &[])
        .with(ProcessInputs, "ProcessInputs", &[])
        .with(Spawner::new(), "Spawner", &["ProcessInputs"])
        .with(SendWorldState, "SendWorldState", &["ProcessInputs"])
        /*
        .with_thread_local(callback!(|time: WriteExpect<ServerTime>| {
            time.advance();
        }))*/;

    Demo::new(SERVER_UPDATE_RATE, world, dispatcher)
}

/*
macro callback {
    (| $($arg:ident: $out:ident<$in:ident>)* | $body:block) => {
        callback!(VeryShort | $($arg: $out<$in>)* | $body)
    },
    ($time:ident | $($arg:ident: $out:ident<$in:ident>)* | $body:block) => {
        {
            struct Sys;
            impl<'a> System<'a> for Sys {
                type SystemData = ($($out<'a, $in>,)*);
                fn run(&mut self, ($(mut $arg,)*): Self::SystemData) {
                    $body
                }
                fn running_time(&self) -> shred::RunningTime {
                    shred::RunningTime::$time
                }
            }
            Sys
        }
    }
}
*/
