use crate::seq::{Seq, SeqBuffer};

mod header;
use self::header::{Header, write_header};

#[derive(Debug)]
pub enum Error {
    PacketTooLarge,
    PacketHeaderInvalid,
    PacketStale { seq: Seq },
    Io(std::io::Error),
}

impl From<std::io::Error> for Error {
    fn from(err: std::io::Error) -> Self {
        Error::Io(err)
    }
}

#[derive(Default)]
pub struct Reliable {
    sequence: Seq,
    sent: SeqBuffer<()>,
    recv: SeqBuffer<()>,
}

impl Reliable {
    pub const MAX_SEND: usize = 1024;
    pub const MAX_RECV: usize = Self::MAX_SEND + Header::MAX;

    pub fn next_sequence(&self) -> Seq { self.sequence }

    pub fn send<F>(&mut self, packet: &[u8], mut transmit: F)
        -> Result<(), Error>
        where F: FnMut(Seq, &mut [u8]) -> Result<(), Error>
    {
        if packet.len() > Self::MAX_SEND {
            return Err(Error::PacketTooLarge);
        }

        let seq = self.sequence;
        self.sequence = self.sequence.next();

        let (ack, ack_bits) = self.recv.generate_ack_bits();
        self.sent.insert(seq, ());

        let mut buf: [u8; Self::MAX_RECV] =
            unsafe { std::mem::uninitialized() };

        let len = unsafe {
            let dst = buf.as_mut_ptr();
            let header = write_header(dst, seq.into(), ack, ack_bits);

            let dst = dst.add(header);
            let src = packet.as_ptr();
            let count = packet.len();
            std::ptr::copy_nonoverlapping(src, dst, count);
            header + count
        };
        transmit(seq, &mut buf[..len])
    }

    pub fn recv<P, A>(&mut self, packet: &mut [u8], mut process: P, acked: A)
        -> Result<(), Error>
        where
            P: FnMut(Seq, &mut [u8]),
            A: FnMut(Seq),
    {
        if packet.len() > Self::MAX_RECV {
            return Err(Error::PacketTooLarge);
        }

        let (header, len) = Header::read(packet)?;

        if !self.recv.test_insert(header.seq) {
            return Err(Error::PacketStale { seq: header.seq.into() });
        }

        self.recv.insert(header.seq, ());
        process(header.seq.into(), &mut packet[len..]);

        header.ack_sequences()
            .filter(|&seq| self.sent.exists(seq))
            .for_each(acked);

        Ok(())
    }
}

#[test]
fn reliable() {
    let mut a = Reliable::default();
    let mut b = Reliable::default();

    let packet = [1, 2, 3, 4];

    a.send(&packet, |s, p| {
        assert_eq!(s, 0.into());
        b.recv(p,
            |_s, p| assert_eq!(p, &packet),
            |_ask| assert!(false))
    }).unwrap();

    a.send(&packet, |s, p| {
        assert_eq!(s, 1.into());
        b.recv(p,
            |_s, p| assert_eq!(p, &packet),
            |ack| assert_eq!(ack, 0.into()))
    }).unwrap();

    b.send(&packet, |s, p| {
        assert_eq!(s, 0.into());
        a.recv(p,
            |_s, p| assert_eq!(p, &packet),
            |ack| assert!(ack == 0.into() || ack == 1.into()))
    }).unwrap();

    b.send(&packet, |s, p| {
        assert_eq!(s, 1.into());
        a.recv(p,
            |_s, p| assert_eq!(p, &packet),
            |ack| assert!(ack == 0.into() || ack == 1.into()))
    }).unwrap();
}
