//! see https://github.com/networkprotocol/netcode.io/blob/master/STANDARD.md

#![allow(dead_code)]
#![feature(assoc_unix_epoch)]

#[macro_use]
extern crate log;
extern crate env_logger;

#[macro_use]
extern crate bitflags;
extern crate byteorder;

#[macro_use]
pub mod utils;
pub mod packet_queue;
pub mod packet;
pub mod addr;
pub mod challenge_token;
pub mod connect_token;
pub mod connect_token_private;
pub mod crypto;
pub mod replay_protection;

//pub mod client;
//pub mod server;

//pub mod connection_token;

pub const VERSION_INFO: [u8; 13] = *b"NETCODE 1.01\0";
pub const VERSION_INFO_BYTES: usize = 13;
pub const MAX_PACKET_BYTES: usize = 1200;
pub const MAX_PACKET_SIZE: usize = 1024;

pub const MAX_PAYLOAD_BYTES: usize = 1100;

const TEST_CLIENT_ID: u64 = 0x1;
const TEST_TIMEOUT_SECONDS: u32 = 15;
const TEST_PROTOCOL_ID: u64 = 0x1122334455667788;


//const TEST_SERVER_PORT:             40000,
//const TEST_CONNECT_TOKEN_EXPIRY   30,
