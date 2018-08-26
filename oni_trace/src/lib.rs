#![feature(decl_macro)]

extern crate time;
#[macro_use]
extern crate serde_json;
#[macro_use]
extern crate lazy_static;

use std::cell::RefCell;
use std::fs::File;
use std::io::BufWriter;
use std::string::String;
use std::sync::Mutex;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::thread;
use time::precise_time_ns;

lazy_static! {
    static ref GLOBAL_PROFILER: Mutex<Profiler> = Mutex::new(Profiler::new());
}

#[macro_export]
pub macro location() {
    $crate::Location { module: module_path!(), file: file!(), line: line!() }
}

#[macro_export]
pub macro oni_instant {
    ($name:expr) => {
        #[cfg(feature = "trace")]
        $crate::instant_thread($name, "",
        $crate::Location { module: module_path!(), file: file!(), line: line!() });
    },
    ($cat:expr => $name:expr) => {
        #[cfg(feature = "trace")]
        $crate::instant_thread($name, $cat,
        $crate::Location { module: module_path!(), file: file!(), line: line!() });
    },
    ([ $($cat:ident)+ ] $name:expr) => {
        #[cfg(feature = "trace")]
        $crate::instant_thread($name, stringify!($($cat,)+),
        $crate::Location { module: module_path!(), file: file!(), line: line!() });
    }
}

#[macro_export]
pub macro oni_instant_force {
    ($name:expr) => {
        $crate::instant_thread($name, "",
        $crate::Location { module: module_path!(), file: file!(), line: line!() });
    },
    ($cat:expr => $name:expr) => {
        $crate::instant_thread($name, $cat,
        $crate::Location { module: module_path!(), file: file!(), line: line!() });
    },
    ([ $($cat:ident)+ ] $name:expr) => {
        $crate::instant_thread($name, stringify!($($cat,)+),
        $crate::Location { module: module_path!(), file: file!(), line: line!() });
    }
}

#[macro_export]
pub macro oni_trace_scope($($name:tt)+) {
    #[cfg(feature = "trace")]
    let _profile_scope = $crate::ProfileScope::new(stringify!($($name)+), location!());
}

#[macro_export]
pub macro oni_trace_scope_force($($name:tt)+) {
    let _profile_scope = $crate::ProfileScope::new(stringify!($($name)+), location!());
}

#[macro_export]
pub macro oni_async_event {
    (start $name:ident [ $($cat:ident)+ ] => $id:expr) => {
        #[cfg(feature = "trace")]
        $crate::push_async($id, stringify!($name), stringify!($($cat,)+), $crate::AsyncKind::Start, location!());
    },
    (instant $name:ident [ $($cat:ident)+ ] => $id:expr) => {
        #[cfg(feature = "trace")]
        $crate::push_async($id, stringify!($name), stringify!($($cat,)+), $crate::AsyncKind::Instant, location!());
    },
    (end $name:ident [ $($cat:ident)+ ] => $id:expr) => {
        #[cfg(feature = "trace")]
        $crate::push_async($id, stringify!($name), stringify!($($cat,)+), $crate::AsyncKind::End, location!());
    }
}

#[macro_export]
pub macro oni_async_event_force {
    (start $name:ident [ $($cat:ident)+ ] => $id:expr) => {
        $crate::push_async($id, stringify!($name), stringify!($($cat,)+), $crate::AsyncKind::Start, location!());
    },
    (instant $name:ident [ $($cat:ident)+ ] => $id:expr) => {
        $crate::push_async($id, stringify!($name), stringify!($($cat,)+), $crate::AsyncKind::Instant, location!());
    },
    (end $name:ident [ $($cat:ident)+ ] => $id:expr) => {
        $crate::push_async($id, stringify!($name), stringify!($($cat,)+), $crate::AsyncKind::End, location!());
    }
}

thread_local!(pub static THREAD_PROFILER: RefCell<Option<ThreadProfiler>> = RefCell::new(None));

#[derive(Copy, Clone)]
struct ThreadId(usize);

struct ThreadInfo {
    name: String,
}

#[derive(Copy, Clone)]
pub struct Location {
    pub module: &'static str,
    pub file: &'static str,
    pub line: u32,
}

struct Sample {
    tid: ThreadId,
    name: &'static str,
    t0: u64,
    t1: u64,
    location: Location,
}

pub enum AsyncKind {
    Start,
    Instant,
    End,
}

struct Async {
    id: usize,
    kind: AsyncKind,
    tid: ThreadId,
    name: &'static str,
    cat: &'static str,
    ts: u64,
    location: Location,
}

enum InstantScope {
    Global,
    Process,
    Thread,
}

impl InstantScope {
    fn as_scope(&self) -> &'static str {
        match self {
            InstantScope::Global => "g",
            InstantScope::Process => "p",
            InstantScope::Thread => "t",
        }
    }
}

struct Instant {
    name: &'static str,
    cat: &'static str,
    ts: u64,
    tid: ThreadId,
    scope: InstantScope,
    location: Location,
}

pub struct ThreadProfiler {
    id: ThreadId,
    sample: Sender<Sample>,
    instant: Sender<Instant>,
    async: Sender<Async>,
}

struct Profiler {
    samples: (Sender<Sample>, Receiver<Sample>),
    instants: (Sender<Instant>, Receiver<Instant>),
    asyncs: (Sender<Async>, Receiver<Async>),
    threads: Vec<ThreadInfo>,
}

impl Profiler {
    fn new() -> Self {
        Self {
            samples: channel(),
            instants: channel(),
            asyncs: channel(),
            threads: Vec::new(),
        }
    }

    fn register_thread(&mut self) {
        let id = ThreadId(self.threads.len());
        let current = thread::current();
        let tid = current.id();
        let name = current.name()
            .map(|s| s.to_string())
            .unwrap_or_else(|| format!("<unnamed-{}-{:?}>", id.0, tid));

        self.register_thread_with_name(name);
    }

    fn register_thread_with_name(&mut self, name: String) {
        let id = ThreadId(self.threads.len());
        self.threads.push(ThreadInfo { name });

        THREAD_PROFILER.with(|profiler| {
            assert!(profiler.borrow().is_none());

            *profiler.borrow_mut() = Some(ThreadProfiler {
                id,
                sample: self.samples.0.clone(),
                instant: self.instants.0.clone(),
                async: self.asyncs.0.clone(),
            });
        });
    }

    fn write_profile_json(&self, filename: &str) {
        // Stop reading samples that are written after
        // write_profile_json() is called.
        let start_time = precise_time_ns();
        let mut data = Vec::new();

        for (i, th) in self.threads.iter().enumerate() {
            data.push(json!({
                "ph": "M",

                "pid": 0,
                "tid": i,
                "name": "thread_name",
                "args": json!({
                    "name": th.name.as_str(),
                })
            }));
        }

        while let Ok(instant) = self.instants.1.try_recv() {
            if instant.ts > start_time {
                break;
            }

            data.push(json!({
                "name": instant.name,
                "cat": instant.cat,
                "pid": 0,
                "tid": instant.tid.0,
                "ph": "i",
                "ts": instant.ts / 1000,
                "s": instant.scope.as_scope(),
                "args": json!({
                    "module": instant.location.module,
                    "file": instant.location.file,
                    "line": instant.location.line,
                })
            }));
        }

        while let Ok(event) = self.asyncs.1.try_recv() {
            if event.ts > start_time {
                break;
            }

            let ph = match event.kind {
                AsyncKind::Start => "b",
                AsyncKind::Instant => "n",
                AsyncKind::End => "e",
            };

            data.push(json!({
                "cat": event.cat,
                "id": event.id,
                "name": event.name,
                "pid": 0,
                "tid": event.tid.0,
                "ph": ph,
                "ts": event.ts / 1000,
                "args": json!({
                    "module": event.location.module,
                    "file": event.location.file,
                    "line": event.location.line,
                })
            }));
        }

        while let Ok(sample) = self.samples.1.try_recv() {
            if sample.t0 > start_time {
                break;
            }

            let t0 = sample.t0 / 1000;
            let t1 = sample.t1 / 1000;

            data.push(json!({
                "pid": 0,
                "tid": sample.tid.0,
                "name": sample.name,
                "ph": "X",
                "ts": t0,
                "dur": t1 - t0,
                "args": json!({
                    "module": sample.location.module,
                    "file": sample.location.file,
                    "line": sample.location.line,
                })
            }));

            /*
            data.push(json!({
                "pid": 0,
                "tid": sample.tid.0,
                "name": sample.name,
                "ph": "B",
                "ts": t0,
                "args": json!({
                    "module": sample.location.module,
                    "file": sample.location.file,
                    "line": sample.location.line,
                })
            }));

            data.push(json!({
                "pid": 0,
                "tid": sample.tid.0,
                "ph": "E",
                "ts": t1
            }));
            */
        }

        let f = BufWriter::new(File::create(filename).unwrap());
        serde_json::to_writer(f, &data).unwrap();
    }
}

#[doc(hidden)]
pub struct ProfileScope {
    name: &'static str,
    location: Location,
    start: u64,
}

impl ProfileScope {
    pub fn new(name: &'static str, location: Location) -> Self {
        let start = precise_time_ns();
        Self { name, location, start }
    }
}

impl Drop for ProfileScope {
    /// When the ProfileScope is dropped it records the
    /// length of time it was alive for and records it
    /// against the Profiler.
    fn drop(&mut self) {
        let end = precise_time_ns();

        THREAD_PROFILER.with(|profiler| match *profiler.borrow() {
            Some(ref profiler) => {
                profiler.sample.send(Sample {
                    tid: profiler.id,
                    name: self.name,
                    location: self.location,
                    t0: self.start, t1: end,
                }).ok();
            }
            None => {
                println!("ERROR: ProfileScope {} on unregistered thread!", self.name);
            }
        });
    }
}

/// Writes the global profile to a specific file.
pub fn write_profile_json(filename: &str) {
    GLOBAL_PROFILER.lock().unwrap().write_profile_json(filename);
}

/// Registers the current thread with the global profiler.
pub fn register_thread() {
    GLOBAL_PROFILER.lock().unwrap().register_thread();
}

pub fn register_thread_with_name<S: ToString>(name: S) {
    GLOBAL_PROFILER.lock().unwrap().register_thread_with_name(name.to_string());
}

pub fn instant_thread(name: &'static str, cat: &'static str, location: Location) {
    let ts = precise_time_ns();

    THREAD_PROFILER.with(|profiler| match *profiler.borrow() {
        Some(ref profiler) => {
            profiler.instant.send(Instant {
                name, cat, location, ts,
                scope: InstantScope::Thread,
                tid: profiler.id,
            }).ok();
        }
        None => {
            println!("ERROR: instant_thread on unregistered thread!");
        }
    });
}

pub fn push_async(id: usize, name: &'static str, cat: &'static str, kind: AsyncKind, location: Location) {
    let ts = precise_time_ns();

    THREAD_PROFILER.with(|profiler| match *profiler.borrow() {
        Some(ref profiler) => {
            profiler.async.send(Async {
                id, name, location, ts, kind,
                cat,
                tid: profiler.id,
            }).ok();
        }
        None => {
            println!("ERROR: instant_thread on unregistered thread!");
        }
    });
}
