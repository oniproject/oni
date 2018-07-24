//! see https://github.com/networkprotocol/netcode.io/blob/master/STANDARD.md
#![allow(dead_code)]
#![feature(assoc_unix_epoch)]

extern crate byteorder;

#[macro_use]
pub mod utils;

pub mod packet;
pub mod addr;
pub mod challenge_token;
pub mod connect_token_private;
pub mod crypto;
pub mod replay_protection;

//pub mod connection_token;

pub const VERSION_INFO: &[u8] = b"NETCODE 1.01\0";
pub const VERSION_INFO_BYTES: usize = 13;
