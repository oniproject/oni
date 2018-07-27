//! see https://github.com/networkprotocol/netcode.io/blob/master/STANDARD.md

#![allow(dead_code)]
#![feature(assoc_unix_epoch)]
#![feature(drain_filter)]
#![feature(iterator_find_map)]

#[macro_use]
extern crate log;
extern crate env_logger;

#[macro_use]
extern crate bitflags;
extern crate byteorder;
extern crate rand;
extern crate slotmap;

#[macro_use]
pub mod utils;
pub mod packet_queue;
pub mod packet;
pub mod addr;
pub mod token;
pub mod crypto;
pub mod replay_protection;

pub mod encryption_manager;
pub mod client;
//pub mod server;
//pub mod simulator;

pub const VERSION_INFO: [u8; 13] = *b"NETCODE 1.01\0";
pub const VERSION_INFO_BYTES: usize = 13;

pub const PACKET_SEND_RATE: u64 = 10;
pub const PACKET_SEND_DELTA: ::std::time::Duration = ::std::time::Duration::from_nanos(1_000_000_000 / PACKET_SEND_RATE);

const TEST_CLIENT_ID: u64 = 0x1;
const TEST_TIMEOUT_SECONDS: u32 = 15;
const TEST_PROTOCOL_ID: u64 = 0x1122334455667788;


//const TEST_SERVER_PORT:             40000,
//const TEST_CONNECT_TOKEN_EXPIRY   30,

use std::net::SocketAddr;

pub trait Socket {
    fn send(&mut self, addr: SocketAddr, packet: &[u8]);
    fn recv(&mut self, packet: &mut [u8]) -> Option<(usize, SocketAddr)>;
}
