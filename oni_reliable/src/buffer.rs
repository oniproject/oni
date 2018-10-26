use generic_array::{ArrayLength, GenericArray};
use generic_array::typenum::{
    Quot, U8, U32, U256,
};
use std::mem::replace;
use std::hint::unreachable_unchecked;
use std::ops::Div;
use super::{Sequence, SequenceOps, bitset::BitSet};

pub type Entry<S, T> = (Sequence<S>, T);

pub struct Buffer<T, S = u16, L = U256>
    where S: Eq + Copy, L: ArrayLength<Option<Entry<S, T>>>,
{
    next: Sequence<S>,
    entries: GenericArray<Option<Entry<S, T>>, L>,
}

impl<T, S, L> Default for Buffer<T, S, L>
    where S: Default + Eq + Copy, L: ArrayLength<Option<Entry<S, T>>>,
{
    fn default() -> Self {
        Self {
            next: Sequence::default(),
            entries: GenericArray::default(),
        }
    }
}

impl<T, S, L> Buffer<T, S, L>
    where
        S: Default + Eq + Copy,
        L: ArrayLength<Option<Entry<S, T>>>,
        Sequence<S>: SequenceOps,
{
    pub fn next_available(&self) -> Sequence<S> { self.next }
    pub fn capacity(&self) -> usize { self.entries.len() }

    pub fn reset(&mut self) {
        self.next = Sequence::default();
        for e in &mut self.entries {
            *e = None;
        }
    }

    pub fn remove_all<F>(&mut self, mut callback: F)
        where F: FnMut(Entry<S, T>)
    {
        for e in &mut self.entries {
            if let Some(e) = e.take() {
                callback(e)
            }
        }
    }

    pub fn drain_filter<'a, 'b: 'a, F>(&'b mut self, mut filter: F)
        -> impl Iterator<Item=Entry<S, T>> + 'a
        where F: FnMut(&mut Entry<S, T>) -> bool + 'a
    {
        self.entries.iter_mut().filter_map(move |e| {
            if filter(e.as_mut()?) {
                replace(e, None)
            } else {
                None
            }
        })
    }

    pub fn remove_filter<F>(&mut self, mut callback: F)
        where F: FnMut(&mut Entry<S, T>) -> bool
    {
        for e in &mut self.entries {
            if let Some(entry) = e {
                if callback(entry) {
                    let _ = e.take();
                }
            }
        }
    }

    fn retain_between<F>(&mut self, start: Sequence<S>, finish: Sequence<S>, callback: F)
        where F: FnMut(Entry<S, T>)
    {
        let (start, mut finish) = (start.to_usize(), finish.to_usize());
        if finish < start {
            finish += Sequence::<S>::_HALF.to_usize();
        }
        let range = if finish - start < L::to_usize() {
            start..=finish
        } else {
            0..=L::to_usize()
        };
        // XXX
        range.map(|i| i % L::to_usize())
            .filter_map(|i| unsafe {
                replace(self.entries.get_unchecked_mut(i), None)
            })
            .for_each(callback);
    }

    pub fn can_insert(&self, seq: Sequence<S>) -> bool {
        seq >= self.next.prev_n(L::to_usize())
    }

    fn seq2index(seq: Sequence<S>) -> usize {
        seq.into_index(L::to_usize())
    }

    pub fn get(&self, seq: Sequence<S>) -> Option<&Entry<S, T>> {
        let index = Self::seq2index(seq);
        unsafe { self.entries.get_unchecked(index).as_ref() }
    }
    pub fn get_mut(&mut self, seq: Sequence<S>) -> Option<&mut Entry<S, T>> {
        let index = Self::seq2index(seq);
        unsafe { self.entries.get_unchecked_mut(index).as_mut() }
    }
    pub fn remove(&mut self, seq: Sequence<S>) -> Option<Entry<S, T>> {
        let index = Self::seq2index(seq);
        let entry = unsafe { self.entries.get_unchecked_mut(index) };
        entry.take()
    }
    pub fn replace(&mut self, seq: Sequence<S>, value: T) -> Option<Entry<S, T>> {
        let index = Self::seq2index(seq);
        let entry = unsafe { self.entries.get_unchecked_mut(index) };
        replace(entry, Some((seq, value)))
    }

    pub fn available(&self, seq: Sequence<S>) -> bool {
        self.get(seq).is_none()
    }
    pub fn exists(&self, seq: Sequence<S>) -> bool {
        self.find(seq).is_some()
    }
    pub fn find(&self, seq: Sequence<S>) -> Option<&T> {
        self.get(seq).filter(|(s, _)| *s == seq).map(|(_, v)| v)
    }
    pub fn find_mut(&mut self, seq: Sequence<S>) -> Option<&mut T> {
        self.get_mut(seq).filter(|(s, _)| *s == seq).map(|(_, v)| v)
    }

    pub fn find_or_with<F: FnOnce() -> T>(&mut self, seq: Sequence<S>, f: F) -> &T {
        let index = Self::seq2index(seq);
        match unsafe { self.entries.get_unchecked_mut(index) } {
            Some((s, e)) if *s == seq => e,
            e => {
                replace(e, Some((seq, f())));
                e.as_ref()
                    .map(|(_, e)| e)
                    .unwrap_or_else(|| unsafe { unreachable_unchecked() })
            }
        }
    }

    pub fn create_if<F: FnOnce() -> T>(&mut self, seq: Sequence<S>, f: F) {
        let index = Self::seq2index(seq);
        match unsafe { self.entries.get_unchecked_mut(index) } {
            Some(e) if e.0 == seq => (),
            e => { replace(e, Some((seq, f()))); }
        }
    }

    pub fn insert(&mut self, seq: Sequence<S>, value: T) -> bool {
        self.insert_with(seq, value, |_| ())
    }

    pub fn insert_with<F>(&mut self, seq: Sequence<S>, value: T, callback: F) -> bool
        where F: FnMut(Entry<S, T>)
    {
        if !self.can_insert(seq) {
            return false
        }
        if seq >= self.next {
            self.retain_between(self.next, seq, callback);
            self.next = seq.next();
        }
        self.replace(seq, value);
        true
    }

    pub fn generate_ack_bits<N>(&mut self) -> (Sequence<S>, BitSet<N>)
        where N: ArrayLength<u8> + Div<U8>, Quot<N, U8>: ArrayLength<u8>
    {
        let ack = self.next.prev();
        let mut ack_bits = BitSet::new();
        for i in 0..N::to_usize() {
            if self.exists(ack.prev_n(i)) {
                ack_bits.set(i);
            }
        }
        (ack, ack_bits)
    }
}

impl<T, L: ArrayLength<Option<Entry<u16, T>>>> Buffer<T, u16, L> {
    pub fn generate_ack_bits_u32(&mut self) -> (u16, u32) {
        let (ack, ack_bits) = self.generate_ack_bits::<U32>();
        (ack.into(), unsafe {
            u32::from_le_bytes(std::mem::transmute(ack_bits))
            /*
            let p = ack_bits.as_slice().as_ptr();
            ( as *const u32).read().to_le()
            */
        })
    }
}

#[test]
fn insert() {
    //const TEST_SEQUENCE_BUFFER_SIZE: u16 = 256;

    let mut buf: Buffer<()> = Buffer::default();

    assert!(buf.insert(0.into(), ()));

    assert!(buf.insert(2.into(), ()));
    assert!(buf.insert(3.into(), ()));

    assert!(buf.insert(5.into(), ()));

    buf.retain_between(1.into(), 4.into(), |(s, _)| {
        assert!(s == 2.into() || s == 3.into(), "{:?}", s);
    })

    /*
    assert_eq!(buf.find(1.into()), Some(&()));
    assert_eq!(buf.find(2.into()), Some(&()));
    assert_eq!(buf.find(5.into()), Some(&()));

    buf.insert(TEST_SEQUENCE_BUFFER_SIZE.into(), ());
    assert!(!buf.can_insert(0.into()));
    */
}

#[test]
fn sequence_buffer() {
    const TEST_SEQUENCE_BUFFER_SIZE: u16 = 256;
    #[derive(Clone, PartialEq, Debug)]
    struct Data(Sequence<u16>);

    let mut buf: Buffer<Data> = Buffer::default();
    assert_eq!(buf.next_available(), 0.into());
    assert_eq!(buf.capacity(), TEST_SEQUENCE_BUFFER_SIZE as usize);

    for seq in 0..TEST_SEQUENCE_BUFFER_SIZE {
        let seq = seq.into();
        assert!(buf.find(seq).is_none());
        assert!(buf.find_mut(seq).is_none());
        assert!(buf.can_insert(seq));
        assert!(buf.available(seq));
        assert!(!buf.exists(seq));
    }

    for seq in 0..TEST_SEQUENCE_BUFFER_SIZE * 4 + 1 {
        let seq = seq.into();
        assert!(buf.can_insert(seq));
        assert!(!buf.exists(seq));

        assert!(buf.insert(seq, Data(seq)));
        assert_eq!(buf.next_available(), seq.next());

        assert!(!buf.available(seq));
        assert!(buf.exists(seq));
    }

    for seq in 0..TEST_SEQUENCE_BUFFER_SIZE + 1 {
        let seq = seq.into();
        assert!(!buf.can_insert(seq));
        assert!(!buf.insert(seq, Data(seq)));
        assert!(!buf.available(seq));
        assert!(!buf.exists(seq));
    }

    let mut seq = (TEST_SEQUENCE_BUFFER_SIZE * 4).into();
    for _ in 0..TEST_SEQUENCE_BUFFER_SIZE {
        let mut data = Data(seq);
        assert_eq!(buf.find(seq), Some(&data));
        assert_eq!(buf.find_mut(seq), Some(&mut data));
        assert!(buf.can_insert(seq));
        assert!(!buf.available(seq));
        assert!(buf.exists(seq));
        seq = seq.prev();
    }

    let mut seq = (TEST_SEQUENCE_BUFFER_SIZE * 4).into();
    for _ in 0..TEST_SEQUENCE_BUFFER_SIZE {
        let mut data = Data(seq);
        assert_eq!(buf.find(seq), Some(&data));
        assert_eq!(buf.find_mut(seq), Some(&mut data));
        seq = seq.prev();
    }

    let mut seq = (TEST_SEQUENCE_BUFFER_SIZE * 4).into();
    for _ in 0..TEST_SEQUENCE_BUFFER_SIZE {
        let data = Data(seq);
        assert_eq!(buf.remove(seq), Some((seq, data)));
        seq = seq.prev();
    }

    buf.reset();
    assert_eq!(buf.next_available(), Sequence::from(0));
    assert_eq!(buf.capacity(), TEST_SEQUENCE_BUFFER_SIZE as usize);
    for seq in 0..TEST_SEQUENCE_BUFFER_SIZE {
        let seq = seq.into();
        assert!(buf.find(seq).is_none());
        assert!(buf.find_mut(seq).is_none());
    }
}

#[test]
fn generate_ack_bits() {
    const TEST_SEQUENCE_BUFFER_SIZE: u16 = 256;
    #[derive(Clone, PartialEq, Debug)]
    struct Data;

    let mut buf: Buffer<Data> = Buffer::default();

    let (ack, ack_bits) = buf.generate_ack_bits_u32();
    assert!(ack == 0xFFFF);
    assert!(ack_bits == 0);

    for seq in 0..TEST_SEQUENCE_BUFFER_SIZE + 1 {
        assert!(buf.insert(seq.into(), Data));
    }

    let (ack, ack_bits) = buf.generate_ack_bits_u32();
    assert!(ack == TEST_SEQUENCE_BUFFER_SIZE);
    assert!(ack_bits == 0xFFFFFFFF);

    { // all acks

        buf.reset();

        for &seq in [1, 5, 2, 9, 11].iter() {
            assert!(buf.insert(seq.into(), Data));
        }

        let (ack, ack_bits) = buf.generate_ack_bits_u32();
        assert_eq!(ack, 11);
        assert_eq!(ack_bits, 1 |
                (1 << (11-9)) |
                (1 << (11-2)) |
                (1 << (11-5)) |
                (1 << (11-1)));
    }

    {
        buf.reset();
        let add = 0xFF;

        for &seq in [1, 5, 2, 9, 11 + add].iter() {
            assert!(buf.insert(seq.into(), Data));
        }

        let (ack, ack_bits) = buf.generate_ack_bits_u32();
        assert_eq!(ack, 11 + add);
        assert_eq!(ack_bits, 1);
    }
}
