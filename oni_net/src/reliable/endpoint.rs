use byteorder::{LE, WriteBytesExt};
use std::{
    time::Instant,
    io::Write,
};
use super::{
    Header, Sequence, SequenceBuffer,
};

const MAX_PACKET_SIZE: usize = 1024;
const FRAGMENT_SIZE: usize = 1024;
const FRAGMENT_HEADER_BYTES: usize = 5;
const FRAGMENT_BUFFER_SIZE: usize = FRAGMENT_HEADER_BYTES + Header::MAX_BYTES + FRAGMENT_SIZE;

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
    pub ack_buffer_size: usize,
    pub sent_packets_buffer_size: usize,
    pub received_packets_buffer_size: usize,
    pub fragment_reassembly_buffer_size: usize,

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
            ack_buffer_size: 256,
            sent_packets_buffer_size: 256,
            received_packets_buffer_size: 256,
            fragment_reassembly_buffer_size: 64,

            rtt_smoothing_factor: 0.0025,
            packet_loss_smoothing_factor: 0.1,
            bandwidth_smoothing_factor: 0.1,

            packet_header_size: 28,
        }
    }
}

pub struct Endpoint<C: Callback> {
    callback: C,
    config: Config,
    time: Instant,
    acks: Vec<u16>,
    sequence: Sequence,

    sent: SequenceBuffer<Sent>,
    recv: SequenceBuffer<Recv>,
    reassembly: SequenceBuffer<Frag>,

    counters: Counters,
}

impl<C: Callback> Endpoint<C> {
    pub fn acks(&self) -> &[u16] { &self.acks }
    pub fn next_packet_sequence(&self) -> Sequence { self.sequence }

    pub fn clear_acks(&mut self) {
        self.acks.clear()
    }

    pub fn new(config: &Config, callback: C) -> Self {
        assert!(config.max_packet_size > 0);
        assert!(config.fragment_above > 0);
        //assert!(config.fragment_size > 0);
        assert!(config.ack_buffer_size > 0);
        assert!(config.sent_packets_buffer_size > 0);
        assert!(config.received_packets_buffer_size > 0);

        //assert!(config.transmit_packet_function != NULL );
        //assert!(config.process_packet_function != NULL );

        Self {
            config: config.clone(),
            callback,
            time: Instant::now(),

            sequence: Sequence::default(),

            sent: SequenceBuffer::new(config.sent_packets_buffer_size),
            recv: SequenceBuffer::new(config.received_packets_buffer_size),
            reassembly: SequenceBuffer::new(config.fragment_reassembly_buffer_size),
            acks: Vec::with_capacity(config.ack_buffer_size),

            rtt: 0.0,
            packet_loss: 0.0,
            sent_bandwidth_kbps: 0.0,
            recv_bandwidth_kbps: 0.0,
            acked_bandwidth_kbps: 0.0,

            counters: Counters::default(),
        }
    }

    pub fn send_packet(&mut self, packet: &[u8]) {
        if packet.len() > self.config.max_packet_size {
            self.counters.num_packets_too_large_to_send += 1;
            return;
        }

        let seq = self.sequence;
        self.sequence = self.sequence.next();

        let (ack, ack_bits) = self.recv.generate_ack_bits();
        self.sent.insert(seq, Sent {
            time: self.time,
            packet_bytes: (self.config.packet_header_size + packet.len()) as u32,
            acked: false,
        });

        if packet.len() <= self.config.fragment_above {
            // regular packet
            let mut data = &mut [0u8; MAX_PACKET_SIZE][..];
            let len = {
                let header = Header { seq: seq.into(), ack, ack_bits };
                let len = header.write(data).unwrap();
                let (header, body) = data.split_at_mut(len);
                let body = &mut body[..packet.len()];
                body.copy_from_slice(packet);
                header.len() + body.len()
            };
            self.callback.transmit(seq, &mut data[..len]);
        } else {
            // fragmented packet
            let header = Header { seq: seq.into(), ack, ack_bits };
            let packet_header = &mut [0u8; Header::MAX_BYTES][..];
            let packet_header_bytes = header.write(packet_header).unwrap();

            let num_fragments = packet.len() / FRAGMENT_SIZE + (packet.len() % FRAGMENT_SIZE != 0) as usize;
            assert!(num_fragments >= 1);
            assert!(num_fragments <= self.config.max_fragments as usize);

            let mut q = &packet[..];
            let mut fragment = [0u8; FRAGMENT_BUFFER_SIZE];
            for fragment_id in 0..num_fragments {
                let len = {
                    let mut p = &mut fragment[..];

                    p.write_u8(1).unwrap();
                    p.write_u16::<LE>(seq.into()).unwrap();
                    p.write_u8(fragment_id as u8).unwrap();
                    p.write_u8((num_fragments - 1) as u8).unwrap();

                    if fragment_id == 0 {
                        p.write_all(packet_header).unwrap();
                    }

                    let len = q.len().max(FRAGMENT_SIZE);
                    p.write_all(&q[..len]).unwrap();
                    q = &q[len..];
                    p.len()
                };

                let len = fragment.len() - len;
                self.callback.transmit(seq, &mut fragment[..len]);
                self.counters.num_fragments_sent += 1;
            }
        }

        self.counters.num_packets_sent += 1;
    }

    pub fn receive_packet(&mut self, packet: &[u8]) -> io::Result<()> {
        if packet.len() > self.config.max_packet_size {
            self.counters.num_packets_too_large_to_receive += 1;
            return;
        }

        let prefix_byte = packet_data[0];

        if prefix_byte & 1 == 0 {
            // regular packet

            self.counters.num_packets_received += 1;

            let mut header = Header::default();

            let packet_header_bytes = header.read(packet)?;
            if packet_header_bytes <= 0 {
                self.counters.num_packets_invalid += 1;
                return;
            }

            if !self.received_packets.test_insert(seq) {
                self.counters.num_packets_stale += 1;
                return;
            }

            if !self.config.process_packet_function(seq, &packet[header_bytes..]) {
                return;
            }

            self.received_packets.insert(sequence, Recv {
                time: self.time,
                packet_bytes: self.config.packet_header_size + packet_bytes,
            });

            for i in 0..32 {
                if ack_bits & 1 != 0 {
                    let ack_sequence = ack - (i as u16);
                    if let Some(p) = self.sent.find_mut(ack_sequence).filter(|p| !p.acked) {
                        if self.acks.len() < self.acks.capacity() {
                            self.acks.push(ack_sequence);
                            self.counters.num_packets_acked += 1;
                            packet.acked = true;

                            self.counters.ack_packet(self.time, p.time);
                        }
                    }
                }
                ack_bits >>= 1;
            }
        } else {
            // fragment packet

            int fragment_id;
            int num_fragments;
            int fragment_bytes;

            uint16_t sequence;
            uint16_t ack;
            uint32_t ack_bits;

            if let Some(header) = read_fragment_header(packet
                self.config.max_fragments,
                self.config.fragment_size,
                &fragment_id,
                &num_fragments,
                &fragment_bytes);

            if fragment_header_bytes < 0 {
                self.counters.num_fragments_invalid += 1;
                return;
            }

            let reassembly = self.fragment_reassembly.find(sequence);

            let reassembly = if !reassembly {
                let packet_buffer_size = Header::MAX_BYTES + num_fragments * self.config.fragment_size;
                self.fragment_reassembly.insert(sequence, Frag {
                    sequence,
                    ack: 0,
                    ack_bits: 0,
                    num_fragments_received: 0,
                    num_fragments_total: num_fragments,
                    packet_data: Vec::new(),
                    fragment_received: 0,
                })
            }

            if num_fragments != reassembly.num_fragments_total {
                self.counters.num_fragments_invalid += 1;
                return;
            }

            if reassembly.fragment_received[fragment_id] {
                return;
            }

            reassembly.num_fragments_received += 1;
            reassembly.fragment_received[fragment_id] = true;

            reassembly.store(
                sequence,
                ack,
                ack_bits,
                fragment_id,
                self.config.fragment_size,
                packet_data + fragment_header_bytes,
                packet_bytes - fragment_header_bytes );

            if reassembly.num_fragments_received == reassembly.num_fragments_total {
                self.receive_packet(
                    reassembly_data.packet_data + Header::MAX_BYTES - reassembly_data.packet_header_bytes,
                    reassembly_data.packet_header_bytes + reassembly_data.packet_bytes,
                );
                self.fragment_reassembly.remove_with_cleanup(sequence, fragment_reassembly_data_cleanup);
            }

            self.counters.num_fragments_received += 1;
        }
    }
}

pub struct Counters {
    pub rtt: f32,
    pub packet_loss: f32,
    pub sent_bandwidth_kbps: f32,
    pub recv_bandwidth_kbps: f32,
    pub acked_bandwidth_kbps: f32,

    pub rtt_smoothing_factor: f32,

    pub num_packets_sent: u64,
    pub num_packets_received: u64,
    pub num_packets_acked: u64,
    pub num_packets_stale: u64,
    pub num_packets_invalid: u64,
    pub num_packets_too_large_to_send: u64,
    pub num_packets_too_large_to_receive: u64,

    pub num_fragments_sent: u64,
    pub num_fragments_received: u64,
    pub num_fragments_invalid: u64,
}

impl Counters {
    pub fn ack_packet(&mut self, time: f64, dt: f32) {
        let rtt = 1000.0 * (time - dt as f64) as f32;
        assert!(rtt >= 0.0);
        if (self.rtt == 0.0 && rtt > 0.0) || (self.rtt - rtt).abs() < 0.00001 {
            self.rtt = rtt;
        } else {
            self.rtt += (rtt - self.rtt) * self.rtt_smoothing_factor;
        }
    }
    /*
    pub fn rtt(&self) -> f32 { self.rtt }
    pub fn packet_loss(&self) -> f32 { self.packet_loss }
    pub fn sent_bandwidth_kbps(&self) -> f32 { self.sent_bandwidth_kbps }
    pub fn recv_bandwidth_kbps(&self) -> f32 { self.recv_bandwidth_kbps }
    pub fn acked_bandwidth_kbps(&self) -> f32 { self.acked_bandwidth_kbps }
    pub fn counters(&self) -> &Counters { &self.counters }
    */
}
