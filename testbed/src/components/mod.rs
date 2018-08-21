mod actor;
mod state_buffer;
mod node;
mod net_marker;
mod input_buffer;

pub use self::actor::{Actor, Controller};
pub use self::state_buffer::{State, StateBuffer};
pub use self::node::Node;
pub use self::input_buffer::{InputBuffer, Acks};
pub use self::net_marker::{NetMarker, NetNode, NetNodeBuilder};

use specs::prelude::*;

#[derive(Component)]
#[storage(DenseVecStorage)]
pub struct Conn(pub std::net::SocketAddr);
