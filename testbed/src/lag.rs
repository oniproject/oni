use std::{
    cell::RefCell,
    time::{Duration, Instant},
    sync::{
        Arc, Mutex,
        atomic::{AtomicUsize, Ordering},
    },
    mem::size_of,
};

use crate::util::*;

#[derive(Clone)]
pub struct Socket<R: Clone, T: Clone> {
    pub rx: LagNetwork<R>,
    pub tx: LagNetwork<T>,
}

impl<R: Clone, T: Clone> Socket<R, T> {
    pub fn new(rx: LagNetwork<R>, tx: LagNetwork<T>) -> Self {
        Self { rx, tx }
    }

    pub fn send(&mut self, payload: T) {
        self.tx.send(payload)
    }

    pub fn recv(&mut self) -> Option<R> {
        self.rx.recv()
    }
}

struct Message<T> {
    delivery_time: Instant,
    payload: T,
}

struct Inner<T> {
    messages: Vec<Message<T>>,
    lag: Duration,
    bytes: AtomicUsize,
    kbps: Kbps,
    second: Instant,
}

// A message queue with simulated network lag.
#[derive(Clone)]
pub struct LagNetwork<T: Clone>(Arc<Mutex<Inner<T>>>);

impl<T: Clone> LagNetwork<T> {
    pub fn new(lag: Duration) -> Self {
        LagNetwork(Arc::new(Mutex::new(Inner {
            messages: Vec::new(),
            lag,
            bytes: AtomicUsize::new(0),
            kbps: Kbps(0),
            second: Instant::now() + Duration::from_secs(1),
        })))
    }

    pub fn recv_kbps(&self) -> Kbps {
        let mut inner = self.0.lock().unwrap();
        if inner.second <= Instant::now() {
            inner.second += Duration::from_secs(1);
            inner.kbps = Kbps(inner.bytes.swap(0, Ordering::Relaxed))
        }
        inner.kbps
    }

    /// "Send" a message.
    ///
    /// Store each message with the time when it should be
    /// received, to simulate lag.
    pub fn send(&self, payload: T) {
        let mut inner = self.0.lock().unwrap();

        let delivery_time = Instant::now() + inner.lag;
        inner.messages.push(Message { delivery_time, payload });
    }

    /// Returns a "received" message,
    /// or undefined if there are no messages available yet.
    pub fn recv(&self) -> Option<T> {
        let mut inner = self.0.lock().unwrap();

        let now = Instant::now();
        let pos = inner.messages.iter()
            .position(|m| m.delivery_time <= now);
        if let Some(pos) = pos {
            inner.bytes.fetch_add(size_of::<T>(), Ordering::Relaxed);
            Some(inner.messages.remove(pos).payload)
        } else {
            None
        }
    }
}
