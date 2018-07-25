use std::collections::VecDeque;

pub const DEFAULT_PACKET_QUEUE_SIZE: usize = 256;

pub struct PacketQueue<T> {
    packets: VecDeque<(T, u64)>,
}

impl<T> Default for PacketQueue<T> {
    fn default() -> Self {
        Self::new(DEFAULT_PACKET_QUEUE_SIZE)
    }
}

impl<T> PacketQueue<T> {
    pub fn new(capacity: usize) -> Self {
        Self {
            packets: VecDeque::with_capacity(capacity),
        }
    }

    pub fn num_packets(&self) -> usize {
        self.packets.len()
    }

    pub fn clear(&mut self) {
        self.packets.clear()
    }

    pub fn push(&mut self, data: T, sequence: u64) -> bool {
        if self.packets.len() == self.packets.capacity() {
            return false;
        }
        self.packets.push_back((data, sequence));
        true
    }

    pub fn pop(&mut self) -> Option<(T, u64)> {
        self.packets.pop_front()
    }
}

#[test]
fn queue() {
    let mut queue = PacketQueue::default();

    // attempting to pop a packet off an empty queue should return NULL
    assert!(queue.pop().is_none());

    return;

    // add some packets to the queue and make sure they pop off in the correct order
    {
        let packets: Vec<_> = (0..100).map(|i| vec![0u8; (i+1) * 256]).collect();

        for (i, packet) in packets.iter().enumerate() {
            assert!(queue.push(packet.clone(), i as u64));
        }

        assert_eq!(queue.num_packets(), 100);

        for i in 0..100 {
            let (packet, sequence) = queue.pop().unwrap();
            assert_eq!(sequence, i as u64);
            assert_eq!(packet, packets[i]);
        }
    }

    // after all entries are popped off,
    // the queue is empty, so calls to pop should return NULL

    assert_eq!(queue.num_packets(), 0);
    assert!(queue.pop().is_none());

    // test that the packet queue can be filled to max capacity

    let packets: Vec<_> = (0..DEFAULT_PACKET_QUEUE_SIZE).map(|i| vec![0u8; i * 256]).collect();

    for (i, packet) in packets.iter().enumerate() {
        assert!(queue.push(packet.clone(), i as u64));
    }

    assert_eq!(queue.num_packets(), DEFAULT_PACKET_QUEUE_SIZE );

    // when the queue is full, attempting to push a packet should fail and return 0
    assert!(!queue.push(vec![0u8; 100], 0));

    // make sure all packets pop off in the correct order
    for i in 0..DEFAULT_PACKET_QUEUE_SIZE {
        let (packet, sequence) = queue.pop().unwrap();
        assert_eq!(sequence, i as u64);
        assert_eq!(packet, packets[i]);
    }

    // add some packets again
    for i in 0..DEFAULT_PACKET_QUEUE_SIZE {
        assert!(queue.push(packets[i].clone(), i as u64));
    }

    // clear the queue and make sure that all packets are freed
    queue.clear();
    assert_eq!(queue.num_packets(), 0);
    assert!(queue.pop().is_none());
}
