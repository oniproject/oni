use super::{Sequence, SequenceOps, SequenceIO};
use std::mem::{size_of, uninitialized};

#[derive(Debug, PartialEq)]
pub enum Error<S: Eq + Copy = u16> {
    TooLarge,
    TooSmall,
    Stale(Sequence<S>),
}

/// Unreliable sequenced channel
#[derive(Default)]
pub struct Sequenced {
    sequence: Sequence<u16>,
    last_recv: Sequence<u16>,
}

impl Sequenced {
    const HEADER: usize = size_of::<u16>();

    pub const MAX_SEND: usize = 1024;
    pub const MAX_RECV: usize = Self::MAX_SEND + Self::HEADER;

    pub fn next_sequence(&self) -> Sequence<u16> { self.sequence }

    pub fn send<F>(&mut self, packet: &[u8], mut transmit: F)
        -> Result<(), Error<u16>>
        where F: FnMut(Sequence<u16>, &mut [u8]) -> Result<(), Error<u16>>
    {
        if packet.len() > Self::MAX_SEND {
            return Err(Error::TooLarge);
        }

        let seq = self.sequence.fetch_next();

        let mut buf: [u8; Self::MAX_RECV] = unsafe { uninitialized() };

        let len = {
            let (header, body) = &mut buf[..].split_at_mut(Self::HEADER);
            seq.write(header).unwrap();
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

        let seq = Sequence::<u16>::read(&packet[..Self::HEADER]).unwrap();
        if seq < self.last_recv {
            return Err(Error::Stale(seq));
        }

        process(seq, &mut packet[Self::HEADER..])
    }
}

#[test]
fn basic_send() {
    let mut a = Sequenced::default();
    let mut count = 0;
    a.send(&[1, 2, 3], |_, buf| {
        count += 1;
        assert_eq!(buf, &[0, 0, 1, 2, 3]);
        Ok(())
    }).unwrap();

    assert_eq!(count, 1);
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
