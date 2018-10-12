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

#[macro_use] extern crate specs_derive;
#[macro_use] extern crate serde_derive;

mod utils;
mod client;
mod server;
mod server_list;
mod incoming;

pub mod token;
mod protocol;

//pub mod server_system;

pub use crate::{
    client::{Client, State, ConnectingState, Error},
    server::Server,
    utils::{keygen, crypto_random},
    token::{PublicToken, USER, DATA},
    server_list::ServerList,
    incoming::Incoming,
    protocol::{
        Packet,
        MAX_PAYLOAD, MTU,
        KEY, HMAC, NONCE, XNONCE,
        VERSION, VERSION_LEN,
    },
};

/*
pub const IP4_HEADER: usize = 20 + 8;
pub const IP6_HEADER: usize = 40 + 8;
*/
