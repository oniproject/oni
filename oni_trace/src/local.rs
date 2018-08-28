use std::{cell::RefCell, borrow::Cow, sync::mpsc::Sender};
use log::{Record, Level};

use crate::{
    colors,
    trace::{
        Event,
        Args,
        Base,
        Async,
        Flow,
        Instant,
    },
};

thread_local!(pub static LOCAL: RefCell<Option<Local>> = RefCell::new(None));

pub struct Local {
    id: usize,
    tx: Sender<Event>,
}

impl Local {
    pub fn new(id: usize, tx: Sender<Event>) -> Self {
        Self { id, tx }
    }

    pub fn instant_thread(&self, ts: u64, name: &'static str, cat: &'static str, args: Args) {
        self.instant(Instant::Thread, ts, name.into(), Some(cat.into()), args, None);
    }

    pub fn instant(
        &self,
        kind: Instant,
        ts: u64,
        name: Cow<'static, str>,
        cat: Option<Cow<'static, str>>,
        args: Args,
        cname: Option<&'static str>,
    ) {
        self.tx.send(Event::Instant {
            ts: ts / 1000,
            s: match kind {
                Instant::Thread => "t",
                Instant::Process => "p",
                Instant::Global => "g",
            },
            base: Base {
                name,
                cat,
                pid: 0,
                tid: self.id,
                args,
                cname,
            },
        }).ok();
    }

    pub fn flow_event(
        &self,
        kind: Flow,
        ts: u64,
        id: usize,
        name: Cow<'static, str>,
        cat: Option<Cow<'static, str>>,
        args: Args,
        cname: Option<&'static str>,
    ) {
        let ts = ts / 1000;
        let base = Base {
            name, cat, args, cname,
            pid: 0,
            tid: self.id,
        };
        self.tx.send(match kind {
            Flow::Start => Event::FlowStart { base, id, ts },
            Flow::Step  => Event::FlowStep  { base, id, ts },
            Flow::End   => Event::FlowEnd   { base, id, ts },
        }).ok();
    }

    pub fn async_event(
        &self,
        kind: Async,
        ts: u64,
        id: usize,
        name: Cow<'static, str>,
        cat: Option<Cow<'static, str>>,
        scope: Option<Cow<'static, str>>,
        args: Args,
        cname: Option<&'static str>,
    ) {
        let ts = ts / 1000;
        let base = Base {
            name, cat, args, cname,
            pid: 0,
            tid: 0,
            //XXX tid: self.id,
        };

        self.tx.send(match kind {
            Async::Start   => Event::AsyncStart   { base, id, ts, scope },
            Async::Instant => Event::AsyncInstant { base, id, ts, scope },
            Async::End     => Event::AsyncEnd     { base, id, ts, scope },
        }).ok();
    }

    pub fn complete(
        &self,
        start: u64,
        end: u64,
        name: Cow<'static, str>,
        cat: Option<Cow<'static, str>>,
        args: Args,
    ) {
        self.tx.send(Event::Complete {
            ts: start / 1000,
            dur: (end - start) / 1000,
            base: Base {
                name, cat, args,
                pid: 0,
                tid: self.id,
                cname: None,
            },
        }).ok();
    }

    pub fn log(&self, ts: u64, record: &Record) {
        let name = format!("{}", record.args());
        let (cname, cat) = match record.level() {
            Level::Error => (colors::PEACH, "Error"),
            Level::Warn => (colors::TANGERINE, "Warn"),
            Level::Info => (colors::GREEN, "Info"),
            Level::Debug => (colors::BLUE, "Debug"),
            Level::Trace => (colors::GREY, "Trace"),
        };
        self.instant(Instant::Thread, ts, name.into(), Some(cat.into()), Args::Log {
            target: record.target().into(),
            module_path: record.module_path().map(|m| m.into()),
            file: record.file().map(|m| m.into()),
            line: record.line(),
        }, Some(cname));
    }
}
