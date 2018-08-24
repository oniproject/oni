use generic_array::{
    ArrayLength, GenericArray,
    typenum::U256,
};

use std::mem::replace;

use super::{Sequence, SequenceOps};

pub struct Buffer16<T, L: ArrayLength<Option<(Sequence<u16>, T)>> = U256> {
    seq: Sequence<u16>,
    entries: GenericArray<Option<(Sequence<u16>, T)>, L>,
}

impl<T, L: ArrayLength<Option<(Sequence<u16>, T)>>> Default for Buffer16<T, L> {
    fn default() -> Self {
        Self {
            seq: Sequence::default(),
            entries: GenericArray::default(),
        }
    }
}

impl<T, L: ArrayLength<Option<(Sequence<u16>, T)>>> Buffer16<T, L> {
    pub fn seq(&self) -> Sequence<u16> { self.seq }
    pub fn capacity(&self) -> usize { self.entries.len() }

    pub fn reset(&mut self) {
        self.seq = Sequence::default();
        for e in &mut self.entries {
            *e = None;
        }
    }

    pub fn remove_entries(&mut self, start: Sequence<u16>, finish: Sequence<u16>) {
        self.remove_entries_with(start, finish, |_| ());
    }

    pub fn remove_entries_with<F: FnMut((Sequence<u16>, T))>(
        &mut self,
        start: Sequence<u16>,
        finish: Sequence<u16>,
        callback: F,
    ) {
        let start: u16 = start.into();
        let finish: u16 = finish.into();
        let (start, mut finish) = (start as usize, finish as usize);
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
            .filter_map(|i| replace(unsafe {
                self.entries.get_unchecked_mut(i)
            }, None))
            .for_each(callback);
    }

    pub fn test_insert<S: Into<Sequence<u16>>>(&self, seq: S) -> bool {
        let cap = self.capacity() as u16;
        let end: u16 = self.seq.into();
        seq.into() >= end.wrapping_sub(cap).into()
    }

    pub fn insert<S: Into<Sequence<u16>>>(&mut self, seq: S, value: T) -> bool {
        let seq = seq.into();
        if self.test_insert(seq) {
            if seq.next() > self.seq {
                let start = self.seq;
                self.remove_entries(start, seq);
                self.seq = seq.next();
            }
            replace(&mut self.entries[Self::seq2index(seq)], Some((seq, value)));
            true
        } else {
            false
        }
    }

    fn seq2index<S: Into<Sequence<u16>>>(seq: S) -> usize {
        let seq: u16 = seq.into().into();
        seq as usize % L::to_usize()
    }

    pub fn remove<S: Into<Sequence<u16>>>(&mut self, seq: S)
        -> Option<(Sequence<u16>, T)>
    {
        unsafe { self.entries.get_unchecked_mut(Self::seq2index(seq)).take() }
    }

    pub fn available<S: Into<Sequence<u16>>>(&self, seq: S) -> bool {
        self.entries[Self::seq2index(seq)].is_none()
    }

    pub fn exists<S: Into<Sequence<u16>>>(&self, seq: S) -> bool {
        self.find(seq).is_some()
    }

    pub fn find<S: Into<Sequence<u16>>>(&self, seq: S) -> Option<&T> {
        let seq = seq.into();
        unsafe { self.entries.get_unchecked(Self::seq2index(seq)) }
            .as_ref()
            .filter(|(s, _)| *s == seq)
            .map(|(_, v)| v)
    }
    pub fn find_mut<S: Into<Sequence<u16>>>(&mut self, seq: S) -> Option<&mut T> {
        let seq = seq.into();
        unsafe { self.entries.get_unchecked_mut(Self::seq2index(seq)) }
            .as_mut()
            .filter(|(s, _)| *s == seq)
            .map(|(_, v)| v)
    }

    pub fn create_if<F: FnOnce() -> T>(&mut self, seq: Sequence<u16>, f: F) {
        let e = unsafe { self.entries.get_unchecked_mut(Self::seq2index(seq)) };
        match e {
            Some((seq, _)) if seq == seq => (),
            _ => { replace(e, Some((seq, f()))); }
        }
    }

    pub fn at(&self, index: usize) -> Option<&T> {
        self.entries.get(index).and_then(|v| v.as_ref()).map(|(_, v)| v)
    }

    pub fn at_mut(&mut self, index: usize) -> Option<&mut T> {
        self.entries.get_mut(index).and_then(|v| v.as_mut()).map(|(_, v)| v)
    }

    pub fn get(&self, index: usize) -> Option<&(Sequence<u16>, T)> {
        self.entries.get(index).and_then(|v| v.as_ref())
    }
    pub fn get_mut(&mut self, index: usize) -> Option<&mut (Sequence<u16>, T)> {
        self.entries.get_mut(index).and_then(|v| v.as_mut())
    }

    pub fn generate_ack_bits(&mut self) -> (u16, u32) {
        let ack: u16 = self.seq.prev().into();
        let mut ack_bits = 0;
        for i in 0..32 {
            let seq = Sequence::from(ack.wrapping_sub(i));
            if self.exists(seq) {
                ack_bits |= 1 << i;
            }
        }
        (ack, ack_bits)
    }
}

#[test]
fn sequence_buffer() {
    const TEST_SEQUENCE_BUFFER_SIZE: u16 = 256;
    #[derive(Clone, PartialEq, Debug)]
    struct Data {
        seq: u16,
    }

    let mut buf: Buffer16<Data, U256> = Buffer16::default();
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

    let mut buf: Buffer16<Data, U256> = Buffer16::default();

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
