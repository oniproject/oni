mod actor;
mod state_buffer;
mod node;
mod net_marker;

pub use self::actor::{Actor, Controller};
pub use self::state_buffer::{State, StateBuffer};
pub use self::node::Node;

pub use self::net_marker::{NetMarker, NetNode, NetNodeBuilder};

use specs::prelude::*;
use std::net::SocketAddr;
use crate::sequence::Sequence;

#[derive(Component)]
#[storage(DenseVecStorage)]
pub struct LastProcessedInput(pub Sequence<u8>);

#[derive(Component)]
#[storage(DenseVecStorage)]
pub struct Conn(pub SocketAddr);
