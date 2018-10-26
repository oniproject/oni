use std::net::SocketAddr;
use specs::prelude::*;
use oni::SimulatedSocket as Socket;
use oni_reliable::Sequence;
use crate::{
    components::*,
    ai::*,
    input::*,
    consts::*,
    ui::Demo,
};

mod process_inputs;
mod reconciliation;
mod interpolation;
mod process_server_messages;

pub use self::process_inputs::ProcessInputs;
pub use self::reconciliation::Reconciliation;
pub use self::interpolation::Interpolation;
pub use self::process_server_messages::ProcessServerMessages;

pub fn new_client(
    dispatcher: DispatcherBuilder<'static, 'static>,
    mut socket: oni::Client<Socket>,
    server: SocketAddr, is_ai: bool,
) -> Demo
{
    socket.connect(server).unwrap();
    //socket.send_client(Client::Start, server);

    let mut world = World::new();
    world.register::<Actor>();
    world.register::<NetMarker>();
    world.register::<StateBuffer>();
    world.register::<InterpolationMarker>();

    world.add_resource(Sequence::<u16>::default());

    world.add_resource(socket);
    world.add_resource(server);
    world.add_resource(Reconciliation::new());
    world.add_resource(NetNode::new(1..0xFF00));

    if is_ai {
        world.add_resource(AI::new());
    } else {
        world.add_resource::<Stick>(Stick::default());
    }

    let dispatcher = dispatcher
        .with(ProcessServerMessages, "ProcessServerMessages", &[])
        .with(ProcessInputs::new(), "ProcessInputs", &["ProcessServerMessages"])
        .with(Interpolation, "Interpolation", &["ProcessInputs"]);

    Demo::new(CLIENT_UPDATE_RATE, world, dispatcher)
}
