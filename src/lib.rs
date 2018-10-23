// see https://github.com/networkprotocol/netcode.io/blob/master/STANDARD.md

//! A simple protocol for secure client/server connections over UDP.
//
// Later on, there will be a framework for mmo games.

#![warn(
    trivial_casts,
    //trivial_numeric_casts,
    //missing_docs,
    //unused_results,
    unused_qualifications,
    unused_lifetimes,
    unused_labels,
    unused_import_braces,
    unused_extern_crates,
    //unreachable_pub,
    //unsafe_code,
    //elided_lifetimes_in_paths,
    //box_pointers,
)]

#![recursion_limit="1024"]
#![feature(
    crate_visibility_modifier,
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
)]

pub use oni_simulator as simulator;

//#[macro_use] extern crate specs_derive;
//#[macro_use] extern crate serde_derive;

mod client;
mod server;
mod server_list;
mod incoming;
mod replay_protection;

pub mod prefix_varint;
pub mod bitset;
pub mod token;
pub mod protocol;
pub mod crypto;

//pub mod server_system;

pub use crate::{
    replay_protection::ReplayProtection,
    client::{Client, State, ConnectingState, Error},
    server::Server,
    server_list::ServerList,
    incoming::Incoming,
};

/*
pub const IP4_HEADER: usize = 20 + 8;
pub const IP6_HEADER: usize = 40 + 8;
*/



pub fn unix_time() -> u64 {
    use std::time::SystemTime;
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

use std::{io, net::{SocketAddr, UdpSocket}};

pub trait Socket: Sized {
    fn bind(addr: SocketAddr) -> io::Result<Self>;
    fn connect(&self, addr: SocketAddr) -> io::Result<()>;
    fn local_addr(&self) -> io::Result<SocketAddr>;
    fn recv_from(&self, buf: &mut [u8]) -> io::Result<(usize, SocketAddr)>;
    fn send_to(&self, buf: &[u8], addr: SocketAddr) -> io::Result<usize>;
    fn send(&self, buf: &[u8]) -> io::Result<usize>;
    fn recv(&self, buf: &mut [u8]) -> io::Result<usize>;
    fn set_nonblocking(&self, nonblocking: bool) -> io::Result<()>;
}

impl Socket for UdpSocket {
    fn bind(addr: SocketAddr) -> io::Result<Self> {
        UdpSocket::bind(addr)
    }
    fn connect(&self, addr: SocketAddr) -> io::Result<()> {
        UdpSocket::connect(self, addr)
    }
    fn local_addr(&self) -> io::Result<SocketAddr> {
        UdpSocket::local_addr(self)
    }
    fn recv_from(&self, buf: &mut [u8]) -> io::Result<(usize, SocketAddr)> {
        UdpSocket::recv_from(self, buf)
    }
    fn send_to(&self, buf: &[u8], addr: SocketAddr) -> io::Result<usize> {
        UdpSocket::send_to(self, buf, addr)
    }
    fn send(&self, buf: &[u8]) -> io::Result<usize> {
        UdpSocket::send(self, buf)
    }
    fn recv(&self, buf: &mut [u8]) -> io::Result<usize> {
        UdpSocket::recv(self, buf)
    }
    fn set_nonblocking(&self, nonblocking: bool) -> io::Result<()> {
        UdpSocket::set_nonblocking(self, nonblocking)
    }
}
