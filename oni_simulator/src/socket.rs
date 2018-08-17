use std::{
    net::SocketAddr,
    io::{Result, Error, ErrorKind},
    sync::{
        Arc, Mutex,
        atomic::{AtomicUsize, Ordering},
    },
};

use generic_array::ArrayLength;

use crate::{simulator::Inner, payload::Payload, DefaultMTU};

/// Simulated unreliable unordered connectionless UDP-like socket.
pub struct Socket<MTU: ArrayLength<u8> = DefaultMTU> {
    crate simulator: Arc<Mutex<Inner<MTU>>>,
    crate local_addr: SocketAddr,
    // TODO: read/write timeout? Always nonblocking?

    crate send_bytes: AtomicUsize,
    crate recv_bytes: AtomicUsize,
}

impl<MTU: ArrayLength<u8>> Socket<MTU> {
    /// Takes the value of the counter sent bytes and clear counter.
    pub fn take_send_bytes(&self) -> usize {
        self.send_bytes.swap(0, Ordering::Relaxed)
    }

    /// Takes the value of the counter received bytes and clear counter.
    pub fn take_recv_bytes(&self) -> usize {
        self.recv_bytes.swap(0, Ordering::Relaxed)
    }

    /// Returns the socket address that this socket was created from.
    pub fn local_addr(&self) -> SocketAddr {
        self.local_addr
    }

    /// Sends data on the socket to the given address.
    /// On success, returns the number of bytes written.
    ///
    /// This will return an error when the length of `buf` is greater than `MTU`.
    pub fn send_to(&self, buf: &[u8], addr: SocketAddr) -> Result<usize> {
        if buf.len() > MTU::to_usize() {
            let kind = ErrorKind::InvalidInput;
            return Err(Error::new(kind, "message too large to send"));
        }

        self.send_bytes.fetch_add(buf.len(), Ordering::Relaxed);

        let mut sim = self.simulator.lock().unwrap();
        sim.send(self.local_addr, addr, Payload::from(buf));
        Ok(buf.len())
    }

    /// Receives a single datagram message on the socket.
    /// On success, returns the number of bytes read and the origin.
    ///
    /// The function must be called with valid byte array `buf` of sufficient size to hold the message bytes.
    /// If a message is too long to fit in the supplied buffer, excess bytes may be discarded.
    pub fn recv_from(&self, buf: &mut [u8]) -> Result<(usize, SocketAddr)> {
        let mut sim = self.simulator.lock().unwrap();

        let pos = sim.pending.iter().position(|e| e.to == self.local_addr)
            .ok_or_else(|| Error::new(ErrorKind::WouldBlock, "simulator recv empty"))?;
        let entry = sim.pending.remove(pos);

        let len = entry.payload.copy_to(buf);
        self.recv_bytes.fetch_add(len, Ordering::Relaxed);
        Ok((len, entry.from))
    }
}

impl<MTU: ArrayLength<u8>> Drop for Socket<MTU> {
    fn drop(&mut self) {
        let mut sim = self.simulator.lock().unwrap();
        sim.entries.retain(|e| e.to == self.local_addr);
        sim.pending.retain(|e| e.to == self.local_addr);
    }
}
