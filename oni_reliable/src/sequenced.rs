use byteorder::{LE, ByteOrder};
use super::{Sequence, SequenceOps};
use std::mem::size_of;

#[derive(Debug, PartialEq)]
pub enum Error<S: Eq + Copy> {
    TooLarge,
    TooSmall,
    Stale(Sequence<S>),
}

/// Unreliable sequenced channel
#[derive(Default)]
pub struct Sequenced<S: Default + Eq + Copy = u16> {
    sequence: Sequence<S>,
    last_recv: Sequence<S>,
}

impl<S: Default + Eq + Copy> Sequenced<S> {
    pub const HEADER: usize = size_of::<S>();
    pub const MAX_SEND: usize = 1024;
    pub const MAX_RECV: usize = Self::MAX_SEND + Self::HEADER;

    pub fn next_sequence(&self) -> Sequence<S> { self.sequence }
}

impl Sequenced<u16> {
        //S: Default + Eq + Copy,
        //Sequence<S>: SequenceOps,

    pub fn send<F>(&mut self, packet: &[u8], mut transmit: F)
        -> Result<(), Error<u16>>
        where F: FnMut(Sequence<u16>, &mut [u8]) -> Result<(), Error<u16>>
    {
        if packet.len() > Self::MAX_SEND {
            return Err(Error::TooLarge);
        }

        let seq = self.sequence.fetch_next();

        let mut buf: [u8; Self::MAX_RECV] =
            unsafe { std::mem::uninitialized() };

        let len = {
            let (header, body) = &mut buf[..].split_at_mut(Self::HEADER);
            LE::write_u16(header, seq.into());
            (&mut body[..packet.len()]).copy_from_slice(packet);
            Self::HEADER + packet.len()
        };

        transmit(seq, &mut buf[..len])
    }

    pub fn recv<P>(&mut self, packet: &mut [u8], mut process: P)
        -> Result<(), Error<u16>>
        where P: FnMut(Sequence<u16>, &mut [u8]) -> Result<(), Error<u16>>,
    {
        if packet.len() > Self::MAX_RECV {
            return Err(Error::TooLarge);
        }
        if packet.len() < Self::HEADER {
            return Err(Error::TooSmall);
        }

        let seq: Sequence<u16> = LE::read_u16(&packet[..Self::HEADER]).into();
        if seq < self.last_recv {
            return Err(Error::Stale(seq));
        }

        process(seq, &mut packet[Self::HEADER..])
    }
}

#[test]
fn recv_err_too_large() {
    let mut ss = Sequenced::default();
    let mut packet = [0; 9000];
    let e = ss.recv(&mut packet[..], |_, _| Ok(()));
    assert_eq!(e, Err(Error::TooLarge));
}

#[test]
fn recv_err_too_small() {
    let mut ss = Sequenced::default();
    let mut packet = [0];
    let e = ss.recv(&mut packet[..], |_, _| Ok(()));
    assert_eq!(e, Err(Error::TooSmall));
}

#[test]
fn recv_err_stale() {
    let mut ss = Sequenced::default();
    let mut packet = [0x00, 0xFF, 3, 4];
    let e = ss.recv(&mut packet[..], |_, _| Ok(()));
    assert_eq!(e, Err(Error::Stale(0xFF00.into())));
}
