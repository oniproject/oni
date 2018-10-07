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
)]

#[macro_use] extern crate bitflags;

#[macro_use]
pub mod utils;
pub mod packet;
pub mod crypto;
pub mod encryption_manager;
pub mod client;
pub mod server;
pub mod protection;

pub mod sodium;

pub mod token {
    pub use crate::crypto::{
        Private, Public, Challenge,
        TOKEN_DATA,
        generate_connect_token,
        Key, keygen,
    };
}

pub const NUM_DISCONNECT_PACKETS: usize = 10;

pub const VERSION_BYTES: usize = 4;
pub const VERSION: [u8; VERSION_BYTES] = *b"ONI\0";

use std::time::Duration;

pub const PACKET_SEND_RATE: u64 = 10;
pub const PACKET_SEND_DELTA: Duration =
    Duration::from_nanos(1_000_000_000 / PACKET_SEND_RATE);

pub const IP4_HEADER: usize = 20 + 8;
pub const IP6_HEADER: usize = 40 + 8;

pub mod socket {
    use std::{io, net::{SocketAddr, ToSocketAddrs, UdpSocket}};

    pub trait Socket {
        fn local_addr(&self) -> io::Result<SocketAddr>;
        fn send_to(&self, buf: &[u8], addr: SocketAddr) -> io::Result<usize>;
        fn recv_from(&self, buf: &mut [u8]) -> io::Result<(usize, SocketAddr)>;
    }

    pub struct NoSocket;

    impl Socket for NoSocket {
        fn local_addr(&self) -> io::Result<SocketAddr> {
            Ok("0.0.0.0:0".parse().unwrap())
        }
        fn send_to(&self, buf: &[u8], addr: SocketAddr) -> io::Result<usize> {
            Ok(buf.len())
        }
        fn recv_from(&self, buf: &mut [u8]) -> io::Result<(usize, SocketAddr)> {
            Err(io::Error::new(io::ErrorKind::WouldBlock, "no socket"))
        }
    }

    pub struct Udp(UdpSocket);

    impl Udp {
        pub fn new<A: ToSocketAddrs>(addr: A) -> io::Result<Self> {
            let socket = UdpSocket::bind(addr)?;
            socket.set_nonblocking(true)?;
            Ok(Udp(socket))
        }
    }

    impl Socket for Udp {
        fn local_addr(&self) -> io::Result<SocketAddr> {
            self.0.local_addr()
        }
        fn send_to(&self, buf: &[u8], addr: SocketAddr) -> io::Result<usize> {
            self.0.send_to(buf, addr)
        }
        fn recv_from(&self, buf: &mut [u8]) -> io::Result<(usize, SocketAddr)> {
            self.0.recv_from(buf)
        }
    }

    #[test]
    fn create() {
        let s = Udp::new("127.0.0.1:0").expect("couldn't bind to address");
        println!("addr: {:?}", s.local_addr());
        let mut packet = [0u8; 8];
        let err = s.recv_from(&mut packet[..]).unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::WouldBlock);
    }
}
