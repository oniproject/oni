use serde_json::to_writer;
use std::{
    thread,
    io::Write,
    sync::{
        Mutex,
        mpsc::{channel, Receiver, Sender},
        atomic::{AtomicUsize, Ordering},
    },
};
use crate::{
    local::{Local, LOCAL},
    Args,
    Base,
    Event,
};

lazy_static! {
    pub static ref GLOBAL: Mutex<Global> = Mutex::new(Global::new());
}

#[derive(Clone)]
struct Thread {
    name: String,
    pid: usize,
    sort_index: Option<usize>,
}

pub struct Global {
    tx: Sender<Event>,
    rx: Receiver<Event>,
    threads: Vec<Thread>,
    skip: AtomicUsize,
}

impl Global {
    fn new() -> Self {
        let (tx, rx) = channel();
        Self {
            tx, rx,
            threads: Vec::new(),
            skip: AtomicUsize::new(0),
        }
    }

    pub fn create_sender(&self) -> Sender<Event> {
        self.tx.clone()
    }

    pub fn register_thread(&mut self, pid: usize, sort_index: Option<usize>) {
        let id = self.threads.len();
        let current = thread::current();
        let tid = current.id();
        let name = current.name()
            .map(|s| s.to_string())
            .unwrap_or_else(|| format!("<unnamed-{}-{:?}>", id, tid));

        let tid = self.threads.len();
        self.threads.push(Thread {
            name, sort_index, pid,
        });

        LOCAL.with(|local| {
            assert!(local.borrow().is_none());
            *local.borrow_mut() = Some(Local::new(tid, pid, self.tx.clone()));
        });
    }

    pub fn write_profile<W: Write>(&self, mut w: W) {
        // Stop reading samples that are written after
        // write_profile_json() is called.

        self.tx.send(Event::Barrier).ok();

        let skip = self.skip.swap(self.threads.len(), Ordering::Relaxed);

        let names = self.threads.iter()
            .skip(skip)
            .cloned()
            .enumerate()
            .map(|(tid, th)| Event::Meta {
                base: Base {
                    name: "thread_name".into(),
                    tid,
                    pid: th.pid,
                    cat: None,
                    args: Args::Name { name: th.name.into() },
                    cname: None,
                },
            });

        let sort_index = self.threads.iter()
            .skip(skip)
            .cloned()
            .enumerate()
            .filter_map(|(i, th)| th.sort_index.map(|idx| (i, idx, th.pid)))
            .map(|(tid, sort_index, pid)| Event::Meta {
                base: Base {
                    name: "thread_sort_index".into(),
                    tid,
                    pid,
                    cat: None,
                    args: Args::SortIndex { sort_index },
                    cname: None,
                },
            });

        let iter = names
            .chain(sort_index)
            .chain(self.rx.try_iter().take_while(|e| !e.is_barrier()));

        for e in iter {
            to_writer(&mut w, &e).unwrap();
            w.write_all(b",\n").unwrap();
        }

        /*
        while let Ok(event) = self.samples.1.try_recv() {
            if event.t0 > start_time {
                break;
            }

            let t0 = event.t0 / 1000;
            let t1 = event.t1 / 1000;

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
        */
    }
}
