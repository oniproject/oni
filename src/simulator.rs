use generic_array::{GenericArray, typenum::{Sum, U200, U1000, Unsigned}};
use rand::{prelude::*, distributions::{Distribution, Uniform}};
use crossbeam_channel::{Sender, Receiver, unbounded};
use std::{
    cell::Cell,
    time::{Instant, Duration},
    net::SocketAddr,
    io::{Result, Error, ErrorKind},
    sync::{Once, Mutex},
    sync::atomic::{AtomicUsize, AtomicU16, Ordering},
    collections::{HashMap, HashSet},
};

/// By default MTU is 1200 bytes.
pub type MTU = Sum<U1000, U200>;

lazy_static! {
    static ref ALREADY_USED: Mutex<HashSet<SocketAddr>> = Mutex::new(HashSet::default());
    static ref EVENT_QUEUE: (Sender<Event>, Receiver<Event>) = unbounded();
}

static PORT: AtomicU16 = AtomicU16::new(1);
fn generate_port() -> u16 { PORT.fetch_add(1, Ordering::Relaxed) }

static START: Once = Once::new();
fn start() { START.call_once(|| { std::thread::spawn(worker); }); }

#[derive(Clone, PartialEq, Debug)]
struct Datagram {
    from: SocketAddr,
    to: SocketAddr,
    len: usize,
    data: GenericArray<u8, MTU>,
}

impl Datagram {
    fn new(from: SocketAddr, to: SocketAddr, payload: &[u8]) -> Self {
        let mut data: GenericArray<u8, MTU> = unsafe { std::mem::zeroed() };
        let len = payload.len();
        (&mut data[..len]).copy_from_slice(payload);
        Self { from, to, data, len }
    }

    fn copy_to(&self, buf: &mut [u8]) -> usize {
        let payload = &self.data[..self.len];
        let len = self.len.min(buf.len());
        (&mut buf[..len]).copy_from_slice(&payload[..len]);
        len
    }
}


#[derive(Debug, Default, Clone, Copy)]
pub struct Config {
    pub latency: Duration,
    pub jitter: Duration,
    pub loss: f64,
}

pub fn config_socket(from: SocketAddr, to: SocketAddr, config: Option<Config>) {
    EVENT_QUEUE.0.send(Event::Config(from, to, config));
}

enum Event {
    Bind(SocketAddr, Sender<Datagram>),
    Config(SocketAddr, SocketAddr, Option<Config>),
    Close(SocketAddr),
    Datagram(Datagram),
}

fn worker() {
    let events = EVENT_QUEUE.1.clone();

    let ticker = crossbeam_channel::tick(Duration::from_millis(4));

    let percents = Uniform::new(0.0, 100.0);

    let mut rng = SmallRng::from_entropy();
    let mut configs: HashMap<(SocketAddr, SocketAddr), Config> = HashMap::default();
    let mut bindings: HashMap<SocketAddr, Sender<Datagram>> = HashMap::default();
    let mut entries: Vec<(Instant, Datagram)> = Vec::new();

    loop {
        select! {
            recv(ticker, now) => if let Some(now) = now {
                for entry in entries.drain_filter(|e| e.0 <= now) {
                    if let Some(to) = bindings.get(&entry.1.to) {
                        to.send(entry.1);
                    }
                }
            }

            recv(events, e) => match e {
                Some(Event::Bind(addr, ch)) =>  { bindings.insert(addr, ch); }
                Some(Event::Close(addr)) => { bindings.remove(&addr); }

                Some(Event::Config(from, to, config)) => {
                    configs.remove(&(to, from));
                    if let Some(config) = config {
                        configs.insert((from, to), config);
                    } else {
                        configs.remove(&(from, to));
                    }
                }
                Some(Event::Datagram(msg)) => {
                    let (from, to) = (msg.from, msg.to);

                    let config = configs.get(&(from, to)).or_else(|| configs.get(&(to, from)));
                    const ZERO: Duration = Duration::from_secs(0);
                    let now = Instant::now();
                    let delivery_time = if let Some(config) = config {
                        if config.loss > percents.sample(&mut rng) {
                            None
                        } else {
                            let delivery = now + config.latency;
                            if config.jitter == ZERO {
                                Some(delivery)
                            } else {
                                let dt = Uniform::new(ZERO, config.jitter).sample(&mut rng);
                                if rng.gen() {
                                    Some(delivery + dt)
                                } else {
                                    Some(delivery - dt)
                                }
                            }
                        }
                    } else {
                        Some(now)
                    };

                    if let Some(time) = delivery_time {
                        entries.push((time, msg));
                    }
                }
                _ => (),
            },
        }
    }
}

/// A simulated socket.
///
/// # Example
///
/// ```
/// use oni::simulator::Socket;
/// use std::io::ErrorKind;
///
/// let from = Socket::bind("[::1]:0".parse().unwrap()).unwrap();
/// let to   = Socket::bind("[::1]:0".parse().unwrap()).unwrap();
///
/// from.send_to(&[1, 2, 3], to.local_addr()).unwrap();
///
/// std::thread::sleep(std::time::Duration::from_millis(100));
///
/// let mut buf = [0u8; 4];
/// let (bytes, addr) = to.recv_from(&mut buf[..]).unwrap();
/// assert_eq!(bytes, 3);
/// assert_eq!(addr, from.local_addr());
/// assert_eq!(&buf[..bytes], &[1, 2, 3]);
///
/// let err = to.recv_from(&mut buf[..]).unwrap_err();
/// assert_eq!(err.kind(), ErrorKind::WouldBlock);
/// ```
pub struct Socket {
    queue: Receiver<Datagram>,
    sender: Sender<Event>,
    local_addr: SocketAddr,

    send_bytes: AtomicUsize,
    recv_bytes: AtomicUsize,

    connect: Cell<Option<SocketAddr>>,
}

impl Drop for Socket {
    fn drop(&mut self) {
        EVENT_QUEUE.0.send(Event::Close(self.local_addr));
        ALREADY_USED.lock().unwrap().remove(&self.local_addr);
    }
}

impl Socket {
    pub fn new() -> Self {
        let addr = "[::1]:0".parse().unwrap();
        Self::bind(addr).unwrap()
    }

    /// Takes the value of the counter sent bytes and clear counter.
    pub fn take_send_bytes(&self) -> usize {
        self.send_bytes.swap(0, Ordering::Relaxed)
    }

    /// Takes the value of the counter received bytes and clear counter.
    pub fn take_recv_bytes(&self) -> usize {
        self.recv_bytes.swap(0, Ordering::Relaxed)
    }

    pub fn bind(mut addr: SocketAddr) -> Result<Self> {
        start();

        if addr.port() == 0 {
            addr.set_port(generate_port());
        }

        let mut used = ALREADY_USED.lock().unwrap();
        assert!(!used.contains(&addr), "address already used: {}", addr);
        used.insert(addr);

        let (sender, queue) = unbounded();

        EVENT_QUEUE.0.send(Event::Bind(addr, sender));

        Ok(Self {
            local_addr: addr,

            send_bytes: AtomicUsize::new(0),
            recv_bytes: AtomicUsize::new(0),

            queue,
            sender: EVENT_QUEUE.0.clone(),
            connect: Cell::new(None),
        })
    }

    pub fn local_addr(&self) -> SocketAddr {
        self.local_addr
    }

    pub fn connect(&self, addr: SocketAddr) {
        self.connect.set(Some(addr));
    }
    pub fn send(&self, buf: &[u8]) -> Result<usize> {
        self.send_to(buf, self.connect.get().unwrap())
    }
    pub fn recv(&self, buf: &mut [u8]) -> Result<usize> {
        let addr = self.connect.get().unwrap();
        loop {
            let (len, from) = self.recv_from(buf)?;
            if from == addr {
                return Ok(len)
            }
        }
    }

    pub fn send_to(&self, buf: &[u8], addr: SocketAddr) -> Result<usize> {
        if buf.len() <= MTU::to_usize() {
            self.send_bytes.fetch_add(buf.len(), Ordering::Relaxed);
            self.sender.send(Event::Datagram(Datagram::new(self.local_addr, addr, buf)));
            Ok(buf.len())
        } else {
            Err(Error::new(ErrorKind::InvalidInput, "message too large to send"))
        }
    }

    pub fn recv_from(&self, buf: &mut [u8]) -> Result<(usize, SocketAddr)> {
        let payload = self.queue.try_recv()
            .ok_or_else(|| Error::new(ErrorKind::WouldBlock, "simulator recv empty"))?;

        let len = payload.copy_to(buf);
        self.recv_bytes.fetch_add(len, Ordering::Relaxed);
        Ok((len, payload.from))
    }
}
