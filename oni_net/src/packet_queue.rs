use std::mem::replace;
use packet::MAX_PAYLOAD_BYTES;

pub const DEFAULT_PACKET_QUEUE_SIZE: usize = 256;

#[derive(Clone)]
struct Entry {
    sequence: u64,
    len: usize,
    data: [u8; MAX_PAYLOAD_BYTES],
}

impl Entry {
    pub fn new() -> Self {
        Self {
            sequence: 0,
            len: 0,
            data: [0u8; MAX_PAYLOAD_BYTES],
        }
    }
    pub fn store_data(&mut self, sequence: u64, src: &[u8]) {
        let len = self.data.len().min(src.len());
        self.len = len;
        self.sequence = sequence;
        let dst = &mut self.data[..len];
        dst.copy_from_slice(&src[..len]);
    }
    pub fn load_data(&mut self) -> (u64, &mut [u8]) {
        (self.sequence, &mut self.data[..self.len])
    }
}

pub struct PacketQueuePool(Vec<PacketQueue>);

impl PacketQueuePool {
    pub fn new() -> Self {
        PacketQueuePool(Vec::new())
    }
    pub fn spawn(&mut self) -> PacketQueue {
        self.0.pop().unwrap_or_else(|| PacketQueue::new())
    }
    pub fn store(&mut self, q: PacketQueue) {
        self.0.push(q)
    }
}

pub struct PacketQueue {
    buf: Vec<Entry>,
    head: usize,
    tail: usize,
}

impl PacketQueue {
    fn new() -> Self {
        Self {
            buf: vec![Entry::new(); DEFAULT_PACKET_QUEUE_SIZE + 1],
            head: 0,
            tail: 0,
        }
    }

    pub fn clear(&mut self) {
        self.head = 0;
        self.tail = 0;
    }

    pub fn is_empty(&self) -> bool {
        self.head == self.tail
    }
    pub fn is_full(&self) -> bool {
        let next = (self.head + 1) % self.buf.len();
        next == self.tail
    }

    pub fn len(&self) -> usize {
        if self.head > self.tail {
            self.head - self.tail
        } else {
            (self.tail - self.head).saturating_sub(1)
        }
    }

    pub fn push(&mut self, sequence: u64, data: &[u8]) -> bool {
        assert!(data.len() <= MAX_PAYLOAD_BYTES, "{}", data.len());
        self.push_truncated(sequence, data)
    }

    pub fn push_truncated(&mut self, sequence: u64, data: &[u8]) -> bool {
        let next = (self.head + 1) % self.buf.len();
        if next == self.tail {
            false
        } else {
            let idx = replace(&mut self.head, next);
            self.buf[idx].store_data(sequence, data);
            true
        }
    }

    pub fn pop(&mut self) -> Option<(u64, &mut [u8])> {
        if self.head == self.tail {
            return None;
        } else {
            let next = (self.tail + 1) % self.buf.len();
            let idx = replace(&mut self.tail, next);
            Some(self.buf[idx].load_data())
        }
    }
}

#[test]
fn queue() {
    let mut pool = PacketQueuePool::new();
    let mut queue = pool.spawn();

    // attempting to pop a packet off an empty queue should return NULL
    assert!(queue.pop().is_none());

    // add some packets to the queue and make sure they pop off in the correct order
    {
        let packets: Vec<_> = (0..100).map(|i| vec![0u8; (i+1) * 2]).collect();

        for (i, packet) in packets.iter().enumerate() {
            assert!(queue.push(i as u64, &packet[..]));
        }

        assert_eq!(queue.len(), 100);

        for (i, p) in packets.iter().enumerate() {
            let (sequence, packet) = queue.pop().unwrap();
            assert_eq!(sequence, i as u64);
            assert_eq!(packet, &p[..]);
        }
    }

    // after all entries are popped off,
    // the queue is empty, so calls to pop should return None
    assert!(queue.is_empty());
    assert!(queue.pop().is_none());

    // test that the packet queue can be filled to max capacity
    let packets: Vec<_> = (0..DEFAULT_PACKET_QUEUE_SIZE).map(|i| vec![0u8; i]).collect();
    for (i, p) in packets.iter().enumerate() {
        assert!(queue.push(i as u64, &p[..]), "len: {}", queue.len());
    }

    assert!(queue.is_full());

    // when the queue is full, attempting to push a packet should fail
    assert!(!queue.push(0, &vec![0u8; 100]));

    // make sure all packets pop off in the correct order
    for (i, p) in packets.iter().enumerate() {
        let (sequence, packet) = queue.pop().unwrap();
        assert_eq!(sequence, i as u64);
        assert_eq!(packet, &p[..]);
    }

    assert!(queue.pop().is_none());

    // add some packets again
    for (i, p) in packets.iter().enumerate() {
        assert!(queue.push(i as u64, &p[..]), "len: {}", queue.len());
    }

    // clear the queue and make sure that all packets are freed
    queue.clear();
    assert!(queue.is_empty());
    assert!(queue.pop().is_none());
}
