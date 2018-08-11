// ommoe - Oni Massively Multiplayer Online Engine

#[macro_use] extern crate specs_derive;
#[macro_use] extern crate smallvec;

pub mod index;

mod actor;
mod room;
mod replica;
mod system;

pub mod prelude32 {
    pub use crate::index::View;
    pub type Replica = crate::Replica<f32>;
    pub type Actor = crate::Actor<f32>;
    pub type Room = crate::Room<f32>;
    pub type RoomSystem = crate::RoomSystem<f32>;
}

pub mod prelude64 {
    pub use crate::index::View;
    pub type Replica = crate::Replica<f64>;
    pub type Actor = crate::Actor<f64>;
    pub type Room = crate::Room<f64>;
    pub type RoomSystem = crate::RoomSystem<f64>;
}

pub use self::{
    replica::Replica,
    actor::Actor,
    room::Room,
    system::RoomSystem,
    index::View,
};
