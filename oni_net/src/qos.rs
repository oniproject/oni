use generic_array::{ArrayLength, GenericArray};
use typenum::U4;
use std::{
    io,
    time::Duration,
};

pub const MAX_FRAGMENTS: usize = 32;

pub struct Conn(usize);
pub struct Chan(u8);


pub enum Error {
    PacketLarge,
    Io(io::Error),
}

impl From<io::Error> for Error {
    fn from(err: io::Error) -> Self {
        Error::Io(err)
    }
}


struct Buffer {
    send: Vec<[u8; 1234]>,
    recv: Vec<[u8; 1234]>,
}

pub struct Transport<A: ArrayLength<u8>, B: ArrayLength<u8>> {
    acks: GenericArray<u8, A>,
    buf: GenericArray<u8, B>,
    buffer: Buffer,
}

impl<A: ArrayLength<u8>, B: ArrayLength<u8>> Transport<A, B> {
    pub fn send(&mut self, to: Conn, chan: Chan, packet: &[u8]) -> Result<(), Error> {
        Ok(())
    }

    pub fn recv(&mut self) -> Result<Option<Event>, Error> {
        if true {
            Ok(None)
        } else {
            let buf = self.buf.as_mut_slice();
            Ok(Some(Event::Data {
                id: Conn(0),
                chan: Chan(0),
                packet: &buf[..],
            }))
        }
    }
}

pub enum Event<'a> {
    // Data event received.
    // Indicating that data was received.
    Data { id: Conn, chan: Chan, packet: &'a [u8] },
    // Connection event received.
    // Indicating that a new connection was established.
    Connect { id: Conn },
    // Disconnection event received.
    Disconnect { id: Conn },
}

bitflags! {
    pub struct QoS: u8 {
        /// There is no guarantee of delivery or ordering.
        const UNRELIABLE = 0 << 0;
        /// There is no guarantee of delivery or ordering,
        /// but allowing fragmented messages with up to 32 fragments per message.
        const UNRELIABLE_FRAGMENTED = Self::UNRELIABLE.bits | Self::FRAGMENTED.bits;
        /// There is no guarantee of delivery and all unordered messages will be dropped.
        /// Example: VoIP.
        const UNRELIABLE_SEQUENCED = Self::UNRELIABLE.bits | Self::SEQUENCED.bits;
        /// There is garantee of ordering, no guarantee of delivery,
        /// but allowing fragmented messages with up to 32 fragments per message.
        const UNRELIABLE_FRAGMENTED_SEQUENCED =
            Self::UNRELIABLE.bits | Self::FRAGMENTED.bits | Self::SEQUENCED.bits;
        /// An unreliable message.
        /// Only the last message in the send buffer is sent.
        /// Only the most recent message in the receive buffer will be delivered.
        const UNRELIABLE_STATE_UPDATE = 1 << 3 | Self::UNRELIABLE_FRAGMENTED_SEQUENCED.bits;

        /// Each message is guaranteed to be delivered but not guaranteed to be in order.
        const RELIABLE = 1 << 0;
        /// Each message is guaranteed to be delivered,
        /// also allowing fragmented messages with up to 32 fragments per message.
        const RELIABLE_FRAGMENTED = Self::RELIABLE.bits | Self::FRAGMENTED.bits;
        /// Each message is guaranteed to be delivered and in order.
        const RELIABLE_SEQUENCED = Self::RELIABLE.bits | Self::SEQUENCED.bits;
        /// Each message is guaranteed to be delivered in order,
        /// also allowing fragmented messages with up to 32 fragments per message.
        const RELIABLE_FRAGMENTED_SEQUENCED =
            Self::RELIABLE.bits | Self::FRAGMENTED.bits | Self::SEQUENCED.bits;
        /// A reliable message.
        /// Only the last message in the send buffer is sent.
        /// Only the most recent message in the receive buffer will be delivered.
        const RELIABLE_STATE_UPDATE = 1 << 3 | Self::RELIABLE_FRAGMENTED_SEQUENCED.bits;

        const FRAGMENTED = 1 << 1;
        const SEQUENCED = 1 << 2;

        /// A reliable message that will be re-sent with a high frequency until it is acknowledged.
        const ALL_COST_DELIVERY = 1 << 4;
    }
}

pub enum _QoS {
    Unreliable,
    UnreliableFragmented,
    UnreliableSequenced,
    UnreliableFragmentedSequenced,
    UnreliableStateUpdate,

    Reliable,
    ReliableFragmented,
    ReliableSequenced,
    ReliableFragmentedSequenced,
    ReliableStateUpdate,

    AllCostDelivery,
}

pub struct AcksSize {
    bytes: usize,
    // 0x00 -   0 -  0
    // 0x08 -   8 -  1
    // 0x10 -  16 -  2
    // 0x18 -  24 -  3
    // 0x20 -  32 -  4
    // 0x28 -  40 -  5
    // 0x30 -  48 -  6
    // 0x38 -  52 -  7
    // 0x40 -  64 -  8
    // 0x48 -  72 -  9
    // 0x50 -  80 - 10
    // 0x58 -  88 - 11
    // 0x60 -  96 - 12
    // 0x68 - 104 - 13
    // 0x70 - 112 - 14
    // 0x78 - 120 - 15
    // 0x80 - 128 - 16
}

impl AcksSize {
    fn bits(&self) -> usize { self.bytes * 8 }
}

pub struct Config {
    // 33ms
    pub ack_delay: Duration,
    // 32
    pub acks_size: AcksSize,
    // 20ms
    pub all_cost_timeout: Duration,
    // 10ms
    pub min_update_timeout: Duration,
    // 500ms
    pub keep_alive_timeout: Duration,
    // 1200ms
    pub reliable_resend_timeout: Duration,
    // 10ms
    pub send_delay: Duration,
}

/// unreliable channel
pub struct Unreliable;
/// unreliable fragmented channel
pub struct UnreliableLarge {
    drop_timeout: Duration,
}

/// unreliable sequenced channel
pub struct Sequenced {
    sequence: u16,
}
/// unreliable sequenced & fragmented channel
pub struct SequencedLarge {
    drop_timeout: Duration,
    sequence: u16,
}

/// reliable channel
pub struct Reliable;
/// reliable fragmented channel
pub struct ReliableLarge;

/// reliable sequenced channel
pub struct Ordered;
/// reliable sequenced & fragmented channel
pub struct OrderedLarge;
