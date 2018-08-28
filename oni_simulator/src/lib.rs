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

use generic_array::typenum::{Sum, U200, U1000};

/// By default MTU is 1200 bytes.
pub type DefaultMTU = Sum<U1000, U200>;
