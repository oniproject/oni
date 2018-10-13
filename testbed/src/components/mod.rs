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

use oni::reliable::Sequence;
use std::net::SocketAddr;
use specs::prelude::*;

#[derive(Component)]
#[storage(DenseVecStorage)]
pub struct Conn {
    pub addr: SocketAddr,
    pub last_sequence: Sequence<u16>,
}

impl Conn {
    pub fn new(addr: SocketAddr) -> Self {
        Self {
            addr,
            last_sequence: Sequence::default(),
        }
    }
}

#[derive(Component, Default)]
#[storage(NullStorage)]
pub struct InterpolationMarker;
