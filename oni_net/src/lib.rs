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

pub mod packet;
pub mod crypto;
pub mod client;
pub mod protection;
pub mod sodium;

pub mod protocol;
pub mod server;
pub mod server_list;
pub mod token;

pub const NUM_DISCONNECT_PACKETS: usize = 10;

pub const VERSION_BYTES: usize = 4;
pub const VERSION: [u8; VERSION_BYTES] = *b"ONI\0";

use std::time::Duration;

pub const PACKET_SEND_RATE: u64 = 10;
pub const PACKET_SEND_DELTA: Duration =
    Duration::from_nanos(1_000_000_000 / PACKET_SEND_RATE);

/*
pub const IP4_HEADER: usize = 20 + 8;
pub const IP6_HEADER: usize = 40 + 8;
*/
