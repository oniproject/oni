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

#[macro_use] extern crate lazy_static;
#[macro_use] extern crate crossbeam_channel;

//#[macro_use] extern crate specs_derive;
//#[macro_use] extern crate serde_derive;

mod client;
mod server;
mod server_list;
mod incoming;
mod replay_protection;

pub mod simulator;

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
    simulator::Socket as SimulatedSocket,
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
    /// Creates a socket from the given address.
    fn bind(addr: SocketAddr) -> io::Result<Self>;
    /// Returns the socket address that this socket was created from.
    fn local_addr(&self) -> io::Result<SocketAddr>;
    /// Receives a single datagram message on the socket.
    /// On success, returns the number of bytes read and the origin.
    ///
    /// ## Simulated socket
    /// The function must be called with valid byte array `buf` of sufficient size to hold the message bytes.
    /// If a message is too long to fit in the supplied buffer, excess bytes may be discarded.
    fn recv_from(&self, buf: &mut [u8]) -> io::Result<(usize, SocketAddr)>;
    /// Sends data on the socket to the given address.
    /// On success, returns the number of bytes written.
    ///
    /// ## Simulated socket
    /// This will return an error when the length of `buf` is greater than `MTU`.
    fn send_to(&self, buf: &[u8], addr: SocketAddr) -> io::Result<usize>;

    fn connect(&self, addr: SocketAddr) -> io::Result<()>;
    fn send(&self, buf: &[u8]) -> io::Result<usize>;
    fn recv(&self, buf: &mut [u8]) -> io::Result<usize>;

    /// ## Simulated socket
    /// Does nothing.
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

impl Socket for SimulatedSocket {
    fn bind(addr: SocketAddr) -> io::Result<Self> {
        SimulatedSocket::bind(addr)
    }
    fn connect(&self, addr: SocketAddr) -> io::Result<()> {
        SimulatedSocket::connect(self, addr);
        Ok(())
    }
    fn local_addr(&self) -> io::Result<SocketAddr> {
        Ok(SimulatedSocket::local_addr(self))
    }
    fn recv_from(&self, buf: &mut [u8]) -> io::Result<(usize, SocketAddr)> {
        SimulatedSocket::recv_from(self, buf)
    }
    fn send_to(&self, buf: &[u8], addr: SocketAddr) -> io::Result<usize> {
        SimulatedSocket::send_to(self, buf, addr)
    }
    fn send(&self, buf: &[u8]) -> io::Result<usize> {
        SimulatedSocket::send(self, buf)
    }
    fn recv(&self, buf: &mut [u8]) -> io::Result<usize> {
        SimulatedSocket::recv(self, buf)
    }
    fn set_nonblocking(&self, _nonblocking: bool) -> io::Result<()> {
        // nothing
        Ok(())
    }
}
