#![feature(decl_macro)]

#[macro_use] extern crate serde_derive;
#[macro_use] extern crate lazy_static;

use time::precise_time_ns;
use deflate::{
    Compression,
    write::GzEncoder,
};
use std::{
    borrow::Cow,
    time::Duration,
    fs::{remove_file, OpenOptions},
    io::{BufWriter, Write},
    thread::{spawn, sleep, JoinHandle},
    sync::mpsc::{channel, Sender},
    sync::atomic::{AtomicUsize, Ordering},
};

mod scope;
mod local;
mod global;
mod trace;
pub mod colors;

pub use self::scope::ScopeComplete;
pub use self::local::{Local, LOCAL};
pub use self::global::{Global, GLOBAL};
pub use self::trace::{Event, Base, Instant, Async, Args, Flow};

pub const ENABLED: bool = cfg!(feature = "trace");
pub const TRACE_LOC: bool = cfg!(feature = "trace_location");

pub fn generate_id() -> usize {
    static NEXT_ID: AtomicUsize = AtomicUsize::new(0);
    NEXT_ID.fetch_add(1, Ordering::Relaxed)
}

use log::{Log, Metadata, Record};

pub struct Logger;

impl Log for Logger {
    fn enabled(&self, _metadata: &Metadata) -> bool {
        //metadata.level() <= Level::Info
        true
    }

    fn log(&self, record: &Record) {
        let ts = precise_time_ns();
        if self.enabled(record.metadata()) {
            LOCAL.with(|profiler| match *profiler.borrow() {
                Some(ref profiler) => profiler.log(ts, record),
                None => println!("ERROR: push_log on unregistered thread!"),
            });
        }
    }

    fn flush(&self) {}
}

pub struct AppendWorker {
    handle: Option<JoinHandle<()>>,
    tx: Sender<()>,
}

impl AppendWorker {
    pub fn new(filename: &str, duration: Duration) -> Self {
        let _ = remove_file(filename);
        let w = OpenOptions::new()
            .create_new(true)
            .append(true)
            .open(filename)
            .unwrap();

        let (tx, rx) = channel();
        let handle = spawn(move || {
            let encoder = GzEncoder::new(w, Compression::Default);
            let mut buf = BufWriter::new(encoder);
            buf.write(b"[\n").ok();

            loop {
                sleep(duration);
                GLOBAL.lock().unwrap().write_profile(&mut buf);
                buf.flush().ok();
                if rx.try_recv().is_ok() {
                    break;
                }
            }

            write_global_instant(&mut buf, "EOF");

            buf.write(b"]\n").ok();
            buf.flush().ok();
        });

        Self { handle: Some(handle), tx }
    }

    pub fn end(&mut self) {
        self.tx.send(()).unwrap();
        self.handle.take().unwrap().join().unwrap();
    }
}

fn write_global_instant<W: Write>(w: &mut W, name: &'static str) {
    let ts = precise_time_ns();
    serde_json::to_writer(w, &Event::Instant {
        s: "g",
        ts: ts / 1000,
        base: Base {
            name: name.into(),
            cat: None,
            pid: 0,
            tid: 0,
            args: Args::Empty,
            cname: Some(colors::WHITE),
        },
    }).ok();
}

/// Registers the current thread with the global profiler.
pub fn register_thread(sort_index: Option<usize>) {
    GLOBAL.lock().unwrap().register_thread(sort_index);
}

#[macro_export]
pub macro location() {
    if $crate::TRACE_LOC {
        $crate::Args::Location { module: module_path!(), file: file!(), line: line!() }
    } else {
        $crate::Args::Empty
    }
}

#[macro_export]
pub macro instant {
    ($name:expr) => {
        $crate::instant!($name => "");
    },
    ([ $($cat:ident)+ ] $name:expr) => {
        $crate::instant!($name => stringify!($($cat,)+));
    },
    ($cat:expr => $name:expr) => {
        if $crate::ENABLED {
            $crate::instant_thread($name, $cat, $crate::location!());
        }
    }
}

#[macro_export]
pub macro scope($($name:tt)+) {
    let _profile_scope = if $crate::ENABLED {
        Some($crate::ScopeComplete::new(stringify!($($name)+), location!()))
    } else {
        None
    };
}

#[doc(hidden)]
pub fn instant_thread(name: &'static str, cat: &'static str, args: Args) {
    let ts = precise_time_ns();
    LOCAL.with(|profiler| match *profiler.borrow() {
        Some(ref profiler) => profiler.instant_thread(ts, name, cat, args),
        None => println!("ERROR: instant_thread on unregistered thread!"),
    });
}

#[macro_export]
pub macro async_event {
    ($kind:ident, $name:expr, $cat:expr, $id:expr, $cname:expr) => {
        if $crate::ENABLED {
            let cat: Option<&'static str> = $cat;
            $crate::push_async($id, $name, $cat, $crate::Async::$kind, location!(), $cname);
        }
    }
}

#[macro_export]
pub macro async_start {
    ($name:expr, $cat:expr, $id:expr) =>              { $crate::async_event!(Start, $name, $cat, $id, None); },
    ($name:expr, $cat:expr, $id:expr, $cname:expr) => { $crate::async_event!(Start, $name, $cat, $id, Some($cname)); }
}

#[macro_export]
pub macro async_instant {
    ($name:expr, $cat:expr, $id:expr) =>              { $crate::async_event!(Start, $name, $cat, $id, None); },
    ($name:expr, $cat:expr, $id:expr, $cname:expr) => { $crate::async_event!(Start, $name, $cat, $id, Some($cname)); }
}

#[macro_export]
pub macro async_end {
    ($name:expr, $cat:expr, $id:expr) =>              { $crate::async_event!(Start, $name, $cat, $id, None); },
    ($name:expr, $cat:expr, $id:expr, $cname:expr) => { $crate::async_event!(Start, $name, $cat, $id, Some($cname)); }
}

#[doc(hidden)]
pub fn push_async<N, C>(
    id: usize,
    name: N,
    cat: Option<C>,
    kind: Async,
    args: Args,
    cname: Option<&'static str>,
)
    where
        N: Into<Cow<'static, str>>,
        C: Into<Cow<'static, str>>,
{
    let ts = precise_time_ns();
    LOCAL.with(|profiler| match *profiler.borrow() {
        Some(ref profiler) => profiler.async_event(kind, ts, id, name.into(), cat.map(Into::into), None, args, cname),
        None => println!("ERROR: push_async on unregistered thread!"),
    });
}

#[macro_export]
pub macro flow_event {
    ($kind:ident, $name:expr, $cat:expr, $id:expr, $cname:expr) => {
        if $crate::ENABLED {
            let cat: Option<&'static str> = $cat;
            $crate::push_flow($id, $name, cat, $crate::Flow::$kind, location!(), $cname);
        }
    }
}

#[macro_export]
pub macro flow_start {
    ($name:expr, $id:expr) =>              { $crate::flow_event!(Start, $name, None, $id, None); },
    ($name:expr, $id:expr, $cname:expr) => { $crate::flow_event!(Start, $name, None, $id, Some($cname)); }
}

#[macro_export]
pub macro flow_step {
    ($name:expr, $id:expr) =>              { $crate::flow_event!(Step, $name, None, $id, None); },
    ($name:expr, $id:expr, $cname:expr) => { $crate::flow_event!(Step, $name, None, $id, Some($cname)); }
}

#[macro_export]
pub macro flow_end {
    ($name:expr, $id:expr) =>              { $crate::flow_event!(End, $name, None, $id, None); },
    ($name:expr, $id:expr, $cname:expr) => { $crate::flow_event!(End, $name, None, $id, Some($cname)); }
}

#[doc(hidden)]
pub fn push_flow<N, C>(
    id: usize,
    name: N,
    cat: Option<C>,
    kind: Flow,
    args: Args,
    cname: Option<&'static str>,
)
    where
        N: Into<Cow<'static, str>>,
        C: Into<Cow<'static, str>>,
{
    let ts = precise_time_ns();
    LOCAL.with(|profiler| match *profiler.borrow() {
        Some(ref profiler) => profiler.flow_event(kind, ts, id, name.into(), cat.map(Into::into), args, cname),
        None => println!("ERROR: push_flow on unregistered thread!"),
    });
}
