// ommoe - Oni Massively Multiplayer Online Engine

#[macro_use] extern crate specs_derive;
#[macro_use] extern crate smallvec;

pub mod index;

mod components;
mod replica;
mod system;

pub mod prelude32 {
    pub use crate::index::View;
    pub use crate::Spawned;
    pub type Position = crate::Position<f32>;
    pub type Replica = crate::Replica<f32>;
    pub type Room = crate::Room<f32>;
    pub type MultiSystem = crate::MultiSystem<f32>;
}

pub mod prelude64 {
    pub use crate::index::View;
    pub use crate::Spawned;
    pub type Position = crate::Position<f64>;
    pub type Replica = crate::Replica<f64>;
    pub type Room = crate::Room<f64>;
    pub type MultiSystem = crate::MultiSystem<f64>;
}

pub use self::{
    components::{Room, Position, Spawned},
    replica::Replica,
    index::View,
    system::{SingleSystem, MultiSystem},
};
