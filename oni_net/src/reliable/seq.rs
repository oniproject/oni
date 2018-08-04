use std::{
    cmp::Ordering,
    mem::replace,
};

#[repr(transparent)]
#[derive(Debug, Default, Hash, Eq, PartialEq, Clone, Copy)]
pub struct Sequence(u16);

impl From<u16> for Sequence {
    fn from(v: u16) -> Self { Sequence(v) }
}

impl Into<u16> for Sequence {
    fn into(self) -> u16 { self.0 }
}

impl Sequence {
    pub fn next(self) -> Self { Sequence(self.0.wrapping_add(1)) }
    pub fn prev(self) -> Self { Sequence(self.0.wrapping_sub(1)) }
}

impl PartialOrd for Sequence {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> { Some(self.cmp(other)) }
}

impl Ord for Sequence {
    fn cmp(&self, other: &Self) -> Ordering {
        if self.0 == other.0 {
            Ordering::Equal
        } else {
            const HALF: u16 = u16::max_value() / 2;
            if self.0.wrapping_sub(other.0) < HALF {
                Ordering::Greater
            } else {
                Ordering::Less
            }
        }
    }
}

pub struct SequenceBuffer<T> {
    entries: Vec<Option<(Sequence, T)>>,
    seq: Sequence,
}

impl<T: Clone> SequenceBuffer<T> {
    pub fn new(size: usize) -> Self {
        Self {
            seq: Sequence::default(),
            entries: vec![None; size],
        }
    }
}

impl<T> SequenceBuffer<T> {
    pub fn seq(&self) -> Sequence {
        self.seq
    }
    pub fn capacity(&self) -> usize {
        self.entries.len()
    }

    pub fn reset(&mut self) {
        self.seq = Sequence::default();
        for e in &mut self.entries {
            *e = None;
        }
    }

    pub fn remove_entries(&mut self, start: Sequence, finish: Sequence) {
        self.remove_entries_with(start, finish, |_| ());
    }

    pub fn remove_entries_with<F: FnMut((Sequence, T))>(&mut self, start: Sequence, finish: Sequence, callback: F) {
        let (start, mut finish) = (start.0 as usize, finish.0 as usize);
        if finish < start {
            finish += 65535;
        }
        let count = self.capacity();
        let range = if finish - start < count {
            start..=finish
        } else {
            0..=count
        };
        range.map(move |i| i % count)
            .filter_map(|i| replace(unsafe { self.entries.get_unchecked_mut(i) }, None))
            .for_each(callback);
    }

    pub fn test_insert<S: Into<Sequence>>(&self, seq: S) -> bool {
        let cap = self.capacity() as u16;
        seq.into() >= self.seq.0.wrapping_sub(cap).into()
    }

    pub fn insert<S: Into<Sequence>>(&mut self, seq: S, value: T) -> bool {
        let seq = seq.into();
        if self.test_insert(seq) {
            if seq.next() > self.seq {
                let start = self.seq;
                self.remove_entries(start, seq);
                self.seq = seq.next();
            }
            let index = seq.0 as usize % self.capacity();
            replace(&mut self.entries[index], Some((seq, value)));
            true
        } else {
            false
        }
    }

    pub fn remove<S: Into<Sequence>>(&mut self, seq: S) -> Option<(Sequence, T)> {
        let index = seq.into().0 as usize % self.capacity();
        unsafe {
            self.entries.get_unchecked_mut(index).take()
        }
    }

    pub fn available<S: Into<Sequence>>(&self, seq: S) -> bool {
        let index = seq.into().0 as usize % self.capacity();
        self.entries[index].is_none()
    }

    pub fn exists<S: Into<Sequence>>(&self, seq: S) -> bool {
        self.find(seq).is_some()
    }

    pub fn find<S: Into<Sequence>>(&self, seq: S) -> Option<&T> {
        let seq = seq.into();
        let index = seq.0 as usize % self.entries.len();
        unsafe { self.entries.get_unchecked(index) }
            .as_ref()
            .filter(|(s, _)| *s == seq)
            .map(|(_, v)| v)
    }
    pub fn find_mut<S: Into<Sequence>>(&mut self, seq: S) -> Option<&mut T> {
        let seq = seq.into();
        let index = seq.0 as usize % self.entries.len();
        unsafe { self.entries.get_unchecked_mut(index) }
            .as_mut()
            .filter(|(s, _)| *s == seq)
            .map(|(_, v)| v)
    }

    pub fn create_if<F: FnOnce() -> T>(&mut self, seq: Sequence, f: F) {
        let index = seq.0 as usize % self.entries.len();
        let e = unsafe { self.entries.get_unchecked_mut(index) };
        match e {
            &mut Some((seq, ref mut _v)) if seq == seq => (),
            _ => {
                replace(e, Some((seq, f())));
            }
        }
    }

    pub fn at(&self, index: usize) -> Option<&T> {
        match self.get(index) {
            Some(&(_, ref v)) => Some(v),
            _ => None,
        }
    }
    pub fn at_mut(&mut self, index: usize) -> Option<&mut T> {
        match self.get_mut(index) {
            Some(&mut (_, ref mut v)) => Some(v),
            _ => None,
        }
    }
    pub fn get(&self, index: usize) -> Option<&(Sequence, T)> {
        self.entries.get(index).and_then(|v| v.as_ref())
    }
    pub fn get_mut(&mut self, index: usize) -> Option<&mut (Sequence, T)> {
        self.entries.get_mut(index).and_then(|v| v.as_mut())
    }

    pub fn generate_ack_bits(&mut self) -> (u16, u32) {
        let ack = self.seq.prev().0;
        let mut ack_bits = 0;
        for i in 0..32 {
            let seq = Sequence(ack.wrapping_sub(i));
            if self.exists(seq) {
                ack_bits |= 1 << i;
            }
        }
        (ack, ack_bits)
    }
}

#[test]
fn sequence() {
    fn sequence_greater_than(a: u16, b: u16) -> bool {
        const HALF: u16 = u16::max_value() / 2;
        a > b && a - b <= HALF ||
        a < b && b - a  > HALF
    }

    let a = Sequence(0);
    let b = Sequence(0xFFFF);

    assert_eq!(a.prev(), b);
    assert_eq!(b.next(), a);

    let tests = &[
        (1, 0, 0xFFFF),
        (2, 1, 0),
        (3, 2, 1),

        (0xFFFF, 0xFFFF - 1, 0xFFFF - 2),
        (0x0000, 0xFFFF - 0, 0xFFFF - 1),
    ];

    for (a, b, c) in tests.into_iter().cloned() {
        assert!(sequence_greater_than(a, b));
        assert!(sequence_greater_than(b, c));
        assert!(sequence_greater_than(a, c));

        let a = Sequence(a);
        let b = Sequence(b);
        let c = Sequence(c);

        assert!(a > b, "{:?} {:?}", a, b);
        assert!(b > c, "{:?} {:?}", b, c);
        assert!(a > c, "{:?} {:?}", a, c);
        assert!(b < a, "{:?} {:?}", b, a);
        assert!(c < b, "{:?} {:?}", c, b);
        assert!(c < a, "{:?} {:?}", c, a);

        assert!(a >= b, "{:?} {:?}", a, b);
        assert!(b >= c, "{:?} {:?}", b, c);
        assert!(a >= c, "{:?} {:?}", a, c);
        assert!(b <= a, "{:?} {:?}", b, a);
        assert!(c <= b, "{:?} {:?}", c, b);
        assert!(c <= a, "{:?} {:?}", c, a);
    }
}

#[test]
fn sequence_buffer() {
    const TEST_SEQUENCE_BUFFER_SIZE: u16 = 256;
    #[derive(Clone, PartialEq, Debug)]
    struct Data {
        seq: u16,
    }

    let mut buf: SequenceBuffer<Data> = SequenceBuffer::new(TEST_SEQUENCE_BUFFER_SIZE as usize);
    assert_eq!(buf.seq(), 0.into());
    assert_eq!(buf.capacity(), TEST_SEQUENCE_BUFFER_SIZE as usize);

    for seq in 0..TEST_SEQUENCE_BUFFER_SIZE {
        assert!(buf.find(seq).is_none());
        assert!(buf.find_mut(seq).is_none());
        assert!(buf.test_insert(seq));
        assert!(buf.available(seq));
        assert!(!buf.exists(seq));
    }

    for seq in 0..TEST_SEQUENCE_BUFFER_SIZE * 4 + 1 {
        assert!(buf.test_insert(seq));
        assert!(!buf.exists(seq));

        assert!(buf.insert(seq, Data { seq }));
        assert_eq!(buf.seq().0, seq + 1);

        assert!(!buf.available(seq));
        assert!(buf.exists(seq));
    }

    for seq in 0..TEST_SEQUENCE_BUFFER_SIZE + 1 {
        assert!(!buf.test_insert(seq));
        assert!(!buf.insert(seq, Data { seq }));
        assert!(!buf.available(seq));
        assert!(!buf.exists(seq));
    }

    let mut seq = TEST_SEQUENCE_BUFFER_SIZE * 4;
    for _ in 0..TEST_SEQUENCE_BUFFER_SIZE {
        let mut data = Data { seq };
        assert_eq!(buf.find(seq), Some(&data));
        assert_eq!(buf.find_mut(seq), Some(&mut data));
        assert!(buf.test_insert(seq));
        assert!(!buf.available(seq));
        assert!(buf.exists(seq));
        seq -= 1;
    }

    let mut seq = TEST_SEQUENCE_BUFFER_SIZE * 4;
    for _ in 0..TEST_SEQUENCE_BUFFER_SIZE {
        let mut data = Data { seq };
        assert_eq!(buf.find(seq), Some(&data));
        assert_eq!(buf.find_mut(seq), Some(&mut data));
        seq -= 1;
    }

    let mut seq = TEST_SEQUENCE_BUFFER_SIZE * 4;
    for _ in 0..TEST_SEQUENCE_BUFFER_SIZE {
        let data = Data { seq };
        assert_eq!(buf.remove(seq), Some((seq.into(), data)));
        seq -= 1;
    }

    buf.reset();
    assert_eq!(buf.seq(), Sequence::from(0));
    assert_eq!(buf.capacity(), TEST_SEQUENCE_BUFFER_SIZE as usize);
    for i in 0..TEST_SEQUENCE_BUFFER_SIZE {
        assert!(buf.find(i).is_none());
        assert!(buf.find_mut(i).is_none());
    }
}

#[test]
fn generate_ack_bits() {
    const TEST_SEQUENCE_BUFFER_SIZE: u16 = 256;
    #[derive(Clone, PartialEq, Debug)]
    struct Data;

    let mut buf: SequenceBuffer<Data> = SequenceBuffer::new(TEST_SEQUENCE_BUFFER_SIZE as usize);

    let (ack, ack_bits) = buf.generate_ack_bits();
    assert!(ack == 0xFFFF);
    assert!(ack_bits == 0);

    for seq in 0..TEST_SEQUENCE_BUFFER_SIZE + 1 {
        assert!(buf.insert(seq, Data));
    }

    let (ack, ack_bits) = buf.generate_ack_bits();
    assert!(ack == TEST_SEQUENCE_BUFFER_SIZE);
    assert!(ack_bits == 0xFFFFFFFF);

    buf.reset();

    for &seq in [1, 5, 9, 11].iter() {
        assert!(buf.insert(seq, Data));
    }

    let (ack, ack_bits) = buf.generate_ack_bits();
    assert!(ack == 11);
    assert!(ack_bits == 1 | (1<<(11-9)) | (1<<(11-5)) | (1<<(11-1)));
}
