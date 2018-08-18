use std::net::SocketAddr;
use specs::prelude::*;
use oni::simulator::Socket;
use crate::{
    components::*,
    ai::*,
    input::*,
    consts::*,
    util::*,
};

mod process_inputs;
mod reconciliation;
mod interpolation;
mod process_server_messages;

pub use self::process_inputs::ProcessInputs;
pub use self::reconciliation::Reconciliation;
pub use self::interpolation::Interpolation;
pub use self::process_server_messages::ProcessServerMessages;

pub trait Controller {
    fn run(&mut self, position: nalgebra::Point2<f32>)
        -> Option<nalgebra::Isometry2<f32>>;
}

pub fn new_client(socket: Socket, server: SocketAddr, is_ai: bool) -> Demo {
    let mut world = World::new();
    world.register::<Actor>();
    world.register::<NetMarker>();
    world.register::<StateBuffer>();

    world.add_resource(socket);
    world.add_resource(server);
    world.add_resource(Reconciliation::new());
    world.add_resource(NetNode::new(0..2));

    if is_ai {
        world.add_resource::<Option<AI>>(Some(AI::new()));
        world.add_resource::<Option<Stick>>(None);
    } else {
        world.add_resource::<Option<AI>>(None);
        world.add_resource::<Option<Stick>>(Some(Stick::default()));
    }

    Demo::new(CLIENT_UPDATE_RATE, world, DispatcherBuilder::new()
        .with(ProcessServerMessages, "ProcessServerMessages", &[])
        .with(ProcessInputs::new(), "ProcessInputs", &["ProcessServerMessages"])
        .with(Interpolation, "Interpolation", &["ProcessInputs"]))
}
