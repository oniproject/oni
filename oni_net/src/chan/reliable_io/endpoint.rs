use byteorder::{LE, WriteBytesExt};
use typenum::U256;
use std::{
    time::Instant,
    io::{self, Write},
};
use super::{
    Error,
    Header, Regular, Fragment,
    Sequence, SequenceBuffer,
    Counters,
};

const MAX_PACKET_SIZE: usize = 1024;
const FRAGMENT_SIZE: usize = 1024;
const FRAGMENT_BUFFER_SIZE: usize =
    Fragment::BYTES + Regular::MAX_BYTES + FRAGMENT_SIZE;

// fragment_reassembly_data_t
#[derive(Clone)]
struct Frag {
    seq: u16,
    ack: u16,
    ack_bits: u32,
    received: usize,
    total: usize,

    /*
    uint8_t * packet_data,
    int packet_bytes,
    int packet_header_bytes,
    uint8_t fragment_received[256],
    */
}

#[derive(Clone)]
struct Sent {
    time: Instant,
    acked: bool,
    packet_bytes: u16,
}

#[derive(Clone)]
struct Recv {
    time: Instant,
    packet_bytes: u16,
}

#[derive(Clone, Debug)]
pub struct Config {
    pub max_packet_size: usize,
    pub fragment_above: usize,
    pub max_fragments: u8,
    //pub fragment_size: usize,
    pub fragment_reassembly_buffer_size: usize,
    pub ack_buffer_size: usize,

    pub rtt_smoothing_factor: f32,
    pub packet_loss_smoothing_factor: f32,
    pub bandwidth_smoothing_factor: f32,

    // NOTE:
    // UDP over IPv4 = 20 + 8 bytes
    // UDP over IPv6 = 40 + 8 bytes
    pub packet_header_size: usize,
}

pub trait Callback {
    fn transmit(&mut self, seq: Sequence, packet: &mut [u8]);
    fn process(&mut self, seq: Sequence, packet: &mut [u8]) -> bool;
}

impl Default for Config {
    fn default() -> Self {
        Self {
            max_packet_size: 16 * 1024,
            fragment_above: 1024,
            max_fragments: 16,
            //fragment_size: 1024,
            fragment_reassembly_buffer_size: 64,

            ack_buffer_size: 256,

            rtt_smoothing_factor: 0.0025,
            packet_loss_smoothing_factor: 0.1,
            bandwidth_smoothing_factor: 0.1,

            packet_header_size: 28,
        }
    }
}

pub struct Endpoint {
    config: Config,

    time: Instant,
    sequence: Sequence,

    sent: SequenceBuffer<Sent, U256>,
    recv: SequenceBuffer<Recv, U256>,
    reassembly: SequenceBuffer<Frag, U256>,

    acks: Vec<u16>,
    counters: Counters,
}

impl Endpoint {
    pub fn acks(&self) -> &[u16] { &self.acks }
    pub fn next_packet_sequence(&self) -> Sequence { self.sequence }

    pub fn clear_acks(&mut self) {
        self.acks.clear()
    }

    pub fn new(config: &Config) -> Self {
        assert!(config.max_packet_size > 0);
        assert!(config.fragment_above > 0);
        //assert!(config.fragment_size > 0);

        Self {
            config: config.clone(),
            time: Instant::now(),

            sequence: Sequence::default(),

            sent: SequenceBuffer::default(),
            recv: SequenceBuffer::default(),
            reassembly: SequenceBuffer::default(),
            acks: Vec::with_capacity(config.ack_buffer_size),

            counters: Counters::default(),
        }
    }

    pub fn send_packet<F>(&mut self, packet: &[u8], mut transmit: F)
        -> Result<(), Error>
        where F: FnMut(Sequence, &[u8])
    {
        if packet.len() > self.config.max_packet_size {
            self.counters.num_packets_too_large_to_send += 1;
            return Err(Error::PacketTooLargeToSend);
        }

        let seq = self.sequence;
        self.sequence = self.sequence.next();

        let (ack, ack_bits) = self.recv.generate_ack_bits();
        let bytes = self.config.packet_header_size + packet.len();
        self.sent.insert(seq, Sent {
            time: self.time,
            packet_bytes: bytes as u16,
            acked: false,
        });

        let header = &mut [0u8; Regular::MAX_BYTES][..];
        let len = Regular { seq: seq.into(), ack, ack_bits }
            .write(header)
            .unwrap();
        let header = &header[..len];

        if packet.len() <= self.config.fragment_above {
            // regular packet
            let mut data = [0u8; MAX_PACKET_SIZE];
            let len = {
                let mut p = &mut data[..];
                p.write_all(header)?;
                p.write_all(packet)?;
                header.len() + packet.len()
            };
            transmit(seq, &mut data[..len]);
        } else {
            // fragmented packet

            let count = packet.len() / FRAGMENT_SIZE +
                (packet.len() % FRAGMENT_SIZE != 0) as usize;
            assert!(count >= 1);
            assert!(count <= self.config.max_fragments as usize);

            let mut q = &packet[..];
            let mut fragment = [0u8; FRAGMENT_BUFFER_SIZE];
            for id in 0..count {
                let len = {
                    let len = Fragment {
                        seq: seq.into(),
                        id: id as u8,
                        total: (count - 1) as u8,
                    }.write(&mut fragment[..]).unwrap();

                    let mut p = &mut fragment[len..];
                    if id == 0 {
                        p.write_all(header).unwrap();
                    }

                    let len = q.len().max(FRAGMENT_SIZE);
                    p.write_all(&q[..len]).unwrap();
                    q = &q[len..];
                    p.len()
                };

                let len = fragment.len() - len;
                self.counters.num_fragments_sent += 1;
                transmit(seq, &mut fragment[..len]);
            }
        }

        self.counters.num_packets_sent += 1;

        Ok(())
    }

    pub fn recv_packet<F>(&mut self, packet: &mut [u8], mut process: F)
        -> Result<(), Error>
        where F: FnMut(Sequence, &mut [u8]) -> bool
    {
        if packet.len() > self.config.max_packet_size {
            self.counters.num_packets_too_large_to_receive += 1;
            return Err(Error::PacketTooLargeToRecv);
        }

        self.counters.num_packets_received += 1;

        if packet[0] & 1 == 0 {
            self.recv_regular(packet, process)
        } else {
            self.recv_fragment(packet, process)
        }
    }

    fn recv_regular<F>(&mut self, packet: &mut [u8], mut process: F)
        -> Result<(), Error>
        where F: FnMut(Sequence, &mut [u8]) -> bool
    {
        let (Regular { seq, ack, ack_bits }, len) = Regular::read(packet)?;

        if !self.recv.test_insert(seq) {
            self.counters.num_packets_stale += 1;
            return Err(Error::PacketStale);
        }

        if !process(seq.into(), &mut packet[len..]) {
            return Ok(());
        }

        let packet_bytes =
            self.config.packet_header_size + packet.len();

        self.recv.insert(seq, Recv {
            time: self.time,
            packet_bytes: packet_bytes as u16,
        });

        let ack_sequences = (0..32)
            .filter(|&i| ack_bits & (1 << i) != 0)
            .map(|i| ack - (i as u16));

        for seq in ack_sequences {
            let p = self.sent.find_mut(seq)
                .filter(|p| !p.acked);
            if let Some(p) = p {
                if self.acks.len() < self.acks.capacity() {
                    self.acks.push(seq);
                    p.acked = true;
                    self.counters.ack_packet(self.time, p.time);
                }
            }
        }

        Ok(())
    }

    fn recv_fragment<F>(&mut self, packet: &mut [u8], mut process: F)
        -> Result<(), Error>
        where F: FnMut(Sequence, &mut [u8]) -> bool
    {
        let (Fragment { seq, id, total }, header_len)
            = Fragment::read(packet)
            .map_err(|_| Error::FragmentHeaderInvalid)?;
        let total = total as usize;

        let is_full = {
            let len = Regular::MAX_BYTES + FRAGMENT_SIZE * total;
            self.reassembly.create_if(seq.into(), || Frag {
                seq,
                ack: 0,
                ack_bits: 0,
                received: 0,
                total,
                packet: vec![0u8; len];
                //received: 0,
            });

            let mut reassembly = self.reassembly.find_mut(seq).unwrap();

            if count != reassembly.total {
                self.counters.num_fragments_invalid += 1;
                return Err(Error::FragmentInvalid);
            }

            if reassembly.fragment_received[id] {
                return Err(Error::FragmentAlreadyReceived);
            }

            reassembly.received += 1;
            reassembly.fragment_received[id] = true;

            reassembly.store(id, &packet[header_len..])?;

            self.counters.num_fragments_received += 1;
            reassembly.received == reassembly.total
        };

        if is_full {
            let reassembly = self.reassembly.remove(seq);
            self.recv_regular(
                reassembly.data
                + Regular::MAX_BYTES - reassembly_data.packet_header_bytes,
                reassembly_data.packet_header_bytes
                + reassembly_data.packet_bytes,
                process,
            )
        } else {
            Ok(())
        }
    }
}
