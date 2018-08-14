#![feature(drain_filter)]
//! # Example
//!
//! ```
//! use oni_simulator::{Simulator, DefaultMTU};
//! use std::io;
//!
//! let sim = Simulator::<DefaultMTU>::new();
//!
//! let from = "[::1]:1111".parse().unwrap();
//! let to   = "[::1]:2222".parse().unwrap();
//!
//! let from = sim.add_socket(from);
//! let to   = sim.add_socket(to);
//!
//! from.send_to(&[1, 2, 3], to.local_addr()).unwrap();
//! sim.advance();
//!
//! let mut buf = [0u8; 4];
//! let (bytes, addr) = to.recv_from(&mut buf[..]).unwrap();
//! assert_eq!(bytes, 3);
//! assert_eq!(addr, from.local_addr());
//! assert_eq!(&buf[..bytes], &[1, 2, 3]);
//!
//! let err = to.recv_from(&mut buf[..]).unwrap_err();
//! assert_eq!(err.kind(), io::ErrorKind::WouldBlock);
//! ```

mod store;
mod config;
mod simulator;
mod socket;
mod payload;

pub use crate::{
    config::Config,
    simulator::Simulator,
    socket::Socket,
};

use generic_array::typenum::{Sum, U8, U20, U40, U80, U200, U500, U576, U1000};

type U1200 = Sum<U1000, U200>;
type U1500 = Sum<U1000, U500>;

pub type DefaultMTU = U1200;
pub type EthernetMTU = U1500;
pub type HeaderIPv4 = U20;
pub type HeaderIPv6 = U40;
pub type HeaderUDP = U8;
pub type IPv4MTUmin = U576;
pub type IPv6MTUmin = Sum<U1200, U80>;
