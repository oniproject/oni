#![feature(decl_macro)]

extern crate deflate;
extern crate log;

extern crate time;
extern crate serde;
extern crate serde_json;
#[macro_use] extern crate serde_derive;
#[macro_use] extern crate lazy_static;

use time::precise_time_ns;
use deflate::{
    Compression,
    write::GzEncoder,
};
use std::{
    thread,
    time::Duration,
    fs::OpenOptions,
    io::{BufWriter, Write},
    sync::mpsc::{channel, Sender},
};

mod scope;
mod local;
mod global;
mod trace;

pub use self::scope::ScopeComplete;
pub use self::local::{Local, LOCAL};
pub use self::global::{Global, GLOBAL};
pub use self::trace::{Event, Base, Instant, Async, Args, Flow};

#[macro_export]
pub macro location() {
    $crate::Args::Location { module: module_path!(), file: file!(), line: line!() }
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
        #[cfg(feature = "trace")]
        $crate::instant_thread($name, $cat, $crate::location!());
    }
}

#[macro_export]
pub macro instant_force {
    ($name:expr) => {
        $crate::instant_force!($name => "");
    },
    ([ $($cat:ident)+ ] $name:expr) => {
        $crate::instant_force!($name => stringify!($($cat,)+));
    },
    ($cat:expr => $name:expr) => {
        $crate::instant_thread($name, $cat, $crate::location!());
    }
}

#[macro_export]
pub macro scope($($name:tt)+) {
    #[cfg(feature = "trace")]
    let _profile_scope = $crate::ScopeComplete::new(stringify!($($name)+), location!());
}

#[macro_export]
pub macro scope_force($($name:tt)+) {
    let _profile_scope = $crate::ScopeComplete::new(stringify!($($name)+), location!());
}

#[macro_export]
pub macro async_event {
    ($kind:ident $name:expr => $cat:expr => $id:expr) => {
        #[cfg(feature = "trace")]
        $crate::push_async($id, $name, $cat, $crate::Async::$kind, location!());
    }
}

#[macro_export]
pub macro async_event_force {
    ($kind:ident $name:expr => $cat:expr => $id:expr) => {
        $crate::push_async($id, $name, $cat, $crate::Async::$kind, location!());
    }
}

#[macro_export]
pub macro flow {
    ($kind:ident $name:expr => $id:expr) => {
        //#[cfg(feature = "trace")]
        $crate::push_flow($id, $name, $crate::Flow::$kind, location!());
    }
}

pub struct AppendWorker {
    handle: Option<thread::JoinHandle<()>>,
    tx: Sender<()>,
}

impl AppendWorker {
    pub fn new(filename: &str, sleep: Duration) -> Self {
        let _ = ::std::fs::remove_file(filename);
        let w = OpenOptions::new()
            .create_new(true)
            .append(true)
            .open(filename)
            .unwrap();

        let (tx, rx) = channel();
        let handle = thread::spawn(move || {
            let encoder = GzEncoder::new(w, Compression::Default);
            let mut buf = BufWriter::new(encoder);
            buf.write(b"[\n").ok();

            loop {
                thread::sleep(sleep);
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
        },
    }).ok();
}

/// Registers the current thread with the global profiler.
pub fn register_thread() {
    GLOBAL.lock().unwrap().register_thread();
}

pub fn instant_thread(name: &'static str, cat: &'static str, args: Args) {
    let ts = precise_time_ns();
    LOCAL.with(|profiler| match *profiler.borrow() {
        Some(ref profiler) => profiler.instant_thread(ts, name, cat, args),
        None => println!("ERROR: instant_thread on unregistered thread!"),
    });
}

pub fn push_async(id: usize, name: &'static str, cat: &'static str, kind: Async, args: Args) {
    let ts = precise_time_ns();
    LOCAL.with(|profiler| match *profiler.borrow() {
        Some(ref profiler) => profiler.async(kind, ts, id, name.into(), Some(cat.into()), None, args),
        None => println!("ERROR: push_async on unregistered thread!"),
    });
}

pub fn push_flow(id: usize, name: &'static str, kind: Flow, args: Args) {
    let ts = precise_time_ns();
    LOCAL.with(|profiler| match *profiler.borrow() {
        Some(ref profiler) => profiler.flow(kind, ts, id, name.into(), None, args),
        None => println!("ERROR: push_async on unregistered thread!"),
    });
}

fn push_log(record: &log::Record) {
    let ts = precise_time_ns();
    LOCAL.with(|profiler| match *profiler.borrow() {
        Some(ref profiler) => profiler.log(ts, record),
        None => println!("ERROR: push_log on unregistered thread!"),
    });
}
