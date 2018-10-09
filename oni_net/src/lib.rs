//! see https://github.com/networkprotocol/netcode.io/blob/master/STANDARD.md

#![recursion_limit="1024"]
#![feature(
    decl_macro,
    drain_filter,
    ptr_offset_from,
    const_fn,
    const_int_ops,
    int_to_from_bytes,
    try_blocks,
    const_let,
    try_from,
    integer_atomics,
    generators
)]

#[macro_use] extern crate bitflags;

#[macro_use]
pub mod utils;

/*
pub mod packet;
pub mod crypto;
pub mod client;
pub mod protection;
pub mod sodium;
*/

pub mod client;

pub mod protocol;
pub mod server;
pub mod server_list;
pub mod token;

/*
pub const IP4_HEADER: usize = 20 + 8;
pub const IP6_HEADER: usize = 40 + 8;
*/
