#![feature(drain_filter, crate_visibility_modifier)]

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

mod simulator;
mod socket;

pub use crate::{simulator::Simulator, socket::Socket};

use generic_array::typenum::{Sum, U200, U1000};
use generic_array::{ArrayLength, GenericArray};
use rand::{prelude::*, distributions::{Distribution, Uniform}};
use std::net::SocketAddr;
use std::time::{Instant, Duration};

/// By default MTU is 1200 bytes.
pub type DefaultMTU = Sum<U1000, U200>;

#[derive(PartialEq, Debug)]
pub struct Datagram<MTU: ArrayLength<u8>> {
    from: SocketAddr,
    to: SocketAddr,
    len: usize,
    data: GenericArray<u8, MTU>,
}

impl<MTU: ArrayLength<u8>> Clone for Datagram<MTU> {
    fn clone(&self) -> Self {
        Self {
            from: self.from,
            to: self.to,
            len: self.len,
            data: self.data.clone(),
        }
    }
}

impl<'a, MTU: ArrayLength<u8>> Datagram<MTU> {
    pub fn new(from: SocketAddr, to: SocketAddr, payload: &'a [u8]) -> Self {
        let mut data: GenericArray<u8, MTU> = unsafe { std::mem::zeroed() };
        let len = payload.len();
        (&mut data[..len]).copy_from_slice(payload);
        Self { from, to, data, len }
    }

    pub fn from(&self) -> &SocketAddr { &self.from }
    pub fn to(&self)   -> &SocketAddr { &self.to   }

    pub fn copy_to(&self, buf: &mut [u8]) -> usize {
        let payload = &self.data[..self.len];
        let len = self.len.min(buf.len());
        (&mut buf[..len]).copy_from_slice(&payload[..len]);
        len
    }
}

const ZERO: Duration = Duration::from_secs(0);
const ONE: Duration = Duration::from_secs(1);

#[derive(Debug, Default, Clone, Copy)]
pub struct Config {
    pub latency: Duration,
    pub jitter: Duration,
    pub loss: f64,
    pub duplicate: f64,
}

impl Config {
    pub fn delivery<R>(&self, rng: &mut R, delivery: Instant) -> Option<Instant>
        where R: Rng + ?Sized
    {
        if self.loss > Uniform::new(0.0, 100.0).sample(rng) {
            return None;
        }

        let delivery = delivery + self.latency;

        if self.jitter == ZERO {
            Some(delivery)
        } else {
            let dt = Uniform::new(ZERO, self.jitter).sample(rng);
            if rng.gen() {
                Some(delivery + dt)
            } else {
                Some(delivery - dt)
            }
        }
    }

    pub fn duplicate<R>(&self, rng: &mut R, delivery: Instant) -> Option<Instant>
        where R: Rng + ?Sized
    {
        if self.duplicate > Uniform::new(0.0, 100.0).sample(rng) {
            Some(delivery + Uniform::new(ZERO, ONE).sample(rng))
        } else {
            None
        }
    }
}
