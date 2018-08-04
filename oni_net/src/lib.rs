//! see https://github.com/networkprotocol/netcode.io/blob/master/STANDARD.md

#![allow(dead_code)]
#![recursion_limit="1024"]
#![feature(
    assoc_unix_epoch,
    drain_filter,
    iterator_find_map,
    rust_2018_preview,
    use_extern_macros,
)]

#[macro_use] extern crate log;
#[macro_use] extern crate bitflags;
#[macro_use] extern crate typenum;

#[macro_use]
pub mod utils;
pub mod version;
pub mod packet;
pub mod addr;
pub mod token;
pub mod crypto;

pub mod encryption_manager;

pub mod client;
//pub mod server;
//pub mod simulator;

pub mod reliable;

pub mod qos;

pub use crate::version::{VERSION, VERSION_BYTES};

pub const PACKET_SEND_RATE: u64 = 10;
pub const PACKET_SEND_DELTA: ::std::time::Duration = ::std::time::Duration::from_nanos(1_000_000_000 / PACKET_SEND_RATE);

pub const IP4_HEADER: usize = 20 + 8;
pub const IP6_HEADER: usize = 40 + 8;

const TEST_CLIENT_ID: u64 = 0x1;
const TEST_TIMEOUT_SECONDS: u32 = 15;
const TEST_PROTOCOL: u64 = 0x1122334455667788;
const TEST_SEQ: u64 = 1000;

//const TEST_SERVER_PORT:             40000,
//const TEST_CONNECT_TOKEN_EXPIRY   30,

use std::net::SocketAddr;

pub trait Socket {
    fn addr(&self) -> SocketAddr;
    fn send(&mut self, addr: SocketAddr, packet: &[u8]);
    fn recv(&mut self, packet: &mut [u8]) -> Option<(usize, SocketAddr)>;
}
