use specs::prelude::*;
use oni::simulator::Socket;
use crate::{
    components::*,
    consts::*,
    ui::Demo,
};

mod process_inputs;
mod send_snapshot;
mod spawner;

use self::process_inputs::ProcessInputs;
use self::send_snapshot::SendWorldState;
use self::spawner::Spawner;

pub fn new_server(pool: std::sync::Arc<rayon::ThreadPool>, network: Socket) -> Demo {
    let mut world = World::new();
    world.register::<Conn>();
    world.register::<Actor>();
    world.register::<NetMarker>();
    world.register::<InputBuffer>();
    world.register::<StateBuffer>();

    world.add_resource(network);
    world.add_resource(NetNode::new(0..2));

    Demo::new(SERVER_UPDATE_RATE, world, DispatcherBuilder::new().with_pool(pool)
        .with(ProcessInputs, "ProcessInputs", &[])
        .with(Spawner::new(), "Spawner", &["ProcessInputs"])
        .with(SendWorldState, "SendWorldState", &["ProcessInputs"]))
}
