#![feature(
    associated_type_defaults,
    decl_macro,
    macro_at_most_once_rep,
    macro_vis_matcher,
)]

// ommoe - Oni Massively Multiplayer Online Engine

#[macro_use]
extern crate specs_derive;

mod entry;
mod index;

mod util;

mod room;
mod replica;

//mod actors;
//mod explosion;

crate use self::entry::Entry;

pub use self::{
    util::{Shim32, Shim64},
    replica::Replica,
    room::{
        Actor, Room, RoomSystem,
    },
    index::{
        SpatialIndex, Around, AroundIndex, Shim,
        kdbush::KDBush,
        brute::Brute,
    },
};
