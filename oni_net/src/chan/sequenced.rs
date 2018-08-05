use byteorder::{LE, ByteOrder};
use super::{Seq, Error};

/// Unreliable sequenced channel
#[derive(Default)]
pub struct Sequenced {
    sequence: Seq,
    last_recv: Seq,
}

impl Sequenced {
    pub const HEADER: usize = 2;
    pub const MAX_SEND: usize = 1024;
    pub const MAX_RECV: usize = Self::MAX_SEND + Self::HEADER;

    pub fn next_sequence(&self) -> Seq { self.sequence }

    pub fn send<F>(&mut self, packet: &[u8], mut transmit: F)
        -> Result<(), Error>
        where F: FnMut(Seq, &mut [u8]) -> Result<(), Error>
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
        -> Result<(), Error>
        where P: FnMut(Seq, &mut [u8]),
    {
        if packet.len() > Self::MAX_RECV {
            return Err(Error::TooLarge);
        }
        if packet.len() < Self::HEADER {
            return Err(Error::TooSmall);
        }

        let seq: Seq = LE::read_u16(&packet[..Self::HEADER]).into();
        if seq < self.last_recv {
            return Err(Error::Stale { seq });
        }

        process(seq, &mut packet[Self::HEADER..]);

        Ok(())
    }
}

#[test]
fn err_invalid_header() {
    let mut ss = Sequenced::default();
    let mut packet = [0x00, 0xFF, 3, 4];
    let e = ss.recv(&mut packet[..], |seq, p| {});
    assert_eq!(e, Err(Error::Stale { seq: 0xFF00.into() }));
}
