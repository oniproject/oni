// chrome-tracing is bad.
// I do my own format.

#![allow(dead_code)]

use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};
use std::num::NonZeroU32;
use std::time::{Instant, Duration};
use std::cell::Cell;
use std::thread::ThreadId;
use std::io::{self, Write};
use byteorder::{LE, WriteBytesExt};
use crossbeam::queue::SegQueue;

type Args = Option<Vec<u8>>;
type Name = Option<String>;

#[repr(u8)]
enum Kind {
    Group,
    Instant,
    AsyncStart,
    AsyncEnd,
    FlowStart,
    FlowEnd,
}

enum Event {
    Barrier,

    Group {
        pid: u32,
        tid: ThreadId,
        group: Group,
        name: Name,
    },

    Instant {
        time: Duration,
        group: Group,
        name: Name,
        args: Args,
    },

    AsyncStart {
        id: ID,
        time: Duration,
        group: Group,
        name: Option<&'static str>,
        args: Args,
    },
    AsyncEnd {
        id: ID,
        time: Duration,
        group: Group,
        args: Args,
    },

    FlowStart {
        id: u64,
        time: Duration,
        group: Group,
        name: Option<&'static str>,
        args: Args,
    },
    FlowEnd {
        id: u64,
        time: Duration,
        group: Group,
        args: Args,
    },
}

lazy_static!(pub static ref GLOBAL: Tracer = Tracer::new(););
thread_local!(pub static LOCAL: Cell<Option<Group>> = Cell::new(None));

pub fn register_thread<S: Into<String>>(name: S) -> Group {
    register_thread_impl(name.into())
}

fn register_thread_impl(name: String) -> Group {
    LOCAL.with(|group| {
        if group.get().is_some() {
            panic!("thread already registred")
        } else {
            let id = GLOBAL.gen_group(Some(name));
            group.set(Some(id));
            id
        }
    })
}

pub fn local_group() -> Group {
    LOCAL.with(|group| {
        if let Some(group) = group.get() {
            group
        } else {
            // register unnamed group
            let id = GLOBAL.gen_group(None);
            group.set(Some(id));
            id
        }
    })
}

#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Group(NonZeroU32);

#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ID(u64);

pub struct Tracer {
    event_id: AtomicU64,
    group_id: AtomicU32,
    start_time: Instant,
    queue: SegQueue<Event>,
}

impl Tracer {
    pub fn new() -> Self {
        Self {
            event_id: AtomicU64::new(1),
            group_id: AtomicU32::new(1),
            start_time: Instant::now(),
            queue: SegQueue::new(),
        }
    }

    #[inline(always)]
    fn elapsed(&self) -> Duration {
        self.start_time.elapsed()
    }

    #[inline(always)]
    fn gen_event_id(&self) -> ID {
        ID(self.event_id.fetch_add(1, Ordering::Relaxed))
    }
    #[inline(always)]
    fn gen_group(&self, name: Option<String>) -> Group {
        let pid = std::process::id();
        let tid = std::thread::current().id();

        let id = self.group_id.fetch_add(1, Ordering::Relaxed);
        let group = Group(unsafe { NonZeroU32::new_unchecked(id) });
        self.queue.push(Event::Group { pid, tid, name, group });
        group
    }

    fn async_start(&self, group: Group, args: Args, name: Option<&'static str>) -> ID {
        let id = self.gen_event_id();
        let time = self.elapsed();
        self.queue.push(Event::AsyncStart { id, group, time, args, name });
        id
    }
    fn async_end(&self, id: ID, group: Group, args: Args) {
        let time = self.elapsed();
        self.queue.push(Event::AsyncEnd { id, time, group, args });
    }

    fn flow_start(&self, id: u64, group: Group, args: Args, name: Option<&'static str>) {
        let time = self.elapsed();
        self.queue.push(Event::FlowStart { id, group, time, args, name });
    }
    fn flow_end(&self, id: u64, group: Group, args: Args) {
        let time = self.elapsed();
        self.queue.push(Event::FlowEnd { id, time, group, args });
    }

    fn write_to<W: Write>(&self, mut w: W) -> io::Result<()> {
        if self.queue.is_empty() {
            return Ok(());
        }
        self.queue.push(Event::Barrier);
        loop {
            match self.queue.try_pop() {
                Some(Event::Barrier) | None => break,
                Some(e) => match e {
                    Event::Barrier => break,
                    Event::Group { pid, tid, group, name } => {
                        let tid: u64 = unsafe { std::mem::transmute(tid) };
                        w.write_u8(Kind::Group as u8)?;
                        w.write_u32::<LE>(group.0.get())?;
                        w.write_u32::<LE>(pid)?;
                        w.write_u64::<LE>(tid)?;
                        write_name(&mut w, name)?;
                    }

                    Event::Instant { time, group, name, args } => {
                        w.write_u8(Kind::Instant as u8)?;
                        w.write_u32::<LE>(group.0.get())?;
                        write_time(&mut w, time)?;
                        write_name(&mut w, name)?;
                        write_args(&mut w, args)?;
                    }

                    Event::FlowStart { group, id, time, name, args } => {
                        w.write_u8(Kind::FlowStart as u8)?;
                        w.write_u32::<LE>(group.0.get())?;
                        w.write_u64::<LE>(id)?;
                        write_time(&mut w, time)?;
                        write_name(&mut w, name)?;
                        write_args(&mut w, args)?;
                    }
                    Event::FlowEnd { group, id, time, args } => {
                        w.write_u8(Kind::FlowEnd as u8)?;
                        w.write_u32::<LE>(group.0.get())?;
                        w.write_u64::<LE>(id)?;
                        write_time(&mut w, time)?;
                        write_args(&mut w, args)?;
                    }

                    Event::AsyncStart { group, id, time, name, args } => {
                        w.write_u8(Kind::AsyncStart as u8)?;
                        w.write_u32::<LE>(group.0.get())?;
                        w.write_u64::<LE>(id.0)?;
                        write_time(&mut w, time)?;
                        write_name(&mut w, name)?;
                        write_args(&mut w, args)?;
                    }
                    Event::AsyncEnd { group, id, time, args } => {
                        w.write_u8(Kind::AsyncEnd as u8)?;
                        w.write_u32::<LE>(group.0.get())?;
                        w.write_u64::<LE>(id.0)?;
                        write_time(&mut w, time)?;
                        write_args(&mut w, args)?;
                    }
                }
            }
        }
        Ok(())
    }
}

fn write_time<W: Write>(w: &mut W, time: Duration) -> io::Result<()> {
    w.write_u64::<LE>(time.as_secs())?;
    w.write_u32::<LE>(time.subsec_nanos())?;
    Ok(())
}

fn write_name<W: Write, S: AsRef<[u8]>>(w: &mut W, name: Option<S>) -> io::Result<()> {
    if let Some(name) = name {
        let name = name.as_ref();
        w.write_u16::<LE>(name.len() as u16)?;
        w.write_all(name)?;
    } else {
        w.write_u16::<LE>(0)?;
    }
    Ok(())
}

fn write_args<W: Write>(w: &mut W, args: Args) -> io::Result<()> {
    if let Some(args) = args {
        w.write_u16::<LE>(args.len() as u16)?;
        w.write_all(&args)?;
    } else {
        w.write_u16::<LE>(0)?;
    }
    Ok(())
}

fn write_header<W: Write>(w: &mut W, version: &str) -> io::Result<()> {
    let padding = 128usize.checked_sub(version.len()).expect("128-byte fixed-size header");
    w.write_all(version.as_bytes())?;
    for _ in 0..padding {
        w.write_u8(0)?;
    }
    Ok(())
}
