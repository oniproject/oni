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

//mod room;
//mod actors;
//mod explosion;
mod replica;

crate use self::{
    entry::Entry,
    util::Iter2,
};

pub use self::{
    util::{Shim32, Shim64},
    replica::Replica,

    index::{
        SpatialIndex, Around, AroundIndex, Shim,
        spatial::SpatialHashMap,
        kdbush::KDBush,
        brute::Brute,
    },
};
