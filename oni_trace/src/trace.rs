use serde_json::Value;
use std::collections::HashMap;
use std::borrow::Cow;

#[derive(Clone, Copy)]
pub enum Async {
    Start,
    Instant,
    End,
}

#[derive(Clone, Copy)]
pub enum Flow {
    Start,
    Step,
    End,
}

#[derive(Clone, Copy)]
pub enum Instant {
    Global,
    Process,
    Thread,
}

#[derive(Serialize)]
#[serde(untagged)]
pub enum Args {
    Empty,
    Name { name: Cow<'static, str> },
    Location {
        module: &'static str,
        file: &'static str,
        line: u32,
    },
    Custom {
        #[serde(flatten)]
        values: HashMap<String, Value>,
    },
    Log {
        target: String,
        module_path: Option<String>,
        file: Option<String>,
        line: Option<u32>,
    },
}

impl Args {
    #[inline]
    pub fn is_empty(&self) -> bool {
        match self {
            Args::Empty => true,
            _ => false,
        }
    }
}

#[derive(Serialize)]
pub struct Base {
    pub name: Cow<'static, str>,
    pub tid: usize,
    pub pid: usize,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub cat: Option<Cow<'static, str>>,
    #[serde(skip_serializing_if = "Args::is_empty")]
    pub args: Args,
}

#[derive(Serialize)]
#[serde(tag = "ph")]
pub enum Event {
    Barrier,

    #[serde(rename = "M")] Meta {
        #[serde(flatten)] base: Base,
    },

    #[serde(rename = "i")] Instant {
        #[serde(flatten)] base: Base,
        s: &'static str,
        ts: u64,
    },

    #[serde(rename = "b")] AsyncStart {
        #[serde(flatten)] base: Base,
        id: usize,
        #[serde(skip_serializing_if = "Option::is_none")]
        scope: Option<Cow<'static, str>>,
        ts: u64,
    },
    #[serde(rename = "n")] AsyncInstant {
        #[serde(flatten)] base: Base,
        id: usize,
        #[serde(skip_serializing_if = "Option::is_none")]
        scope: Option<Cow<'static, str>>,
        ts: u64,
    },
    #[serde(rename = "e")] AsyncEnd {
        #[serde(flatten)] base: Base,
        id: usize,
        #[serde(skip_serializing_if = "Option::is_none")]
        scope: Option<Cow<'static, str>>,
        ts: u64,
    },

    #[serde(rename = "X")] Complete {
        #[serde(flatten)] base: Base,
        dur: u64,
        ts: u64,
    },

    #[serde(rename = "s")] FlowStart {
        #[serde(flatten)] base: Base,
        id: usize,
        ts: u64,
    },
    #[serde(rename = "t")] FlowStep {
        #[serde(flatten)] base: Base,
        id: usize,
        ts: u64,
    },
    #[serde(rename = "f")] FlowEnd {
        #[serde(flatten)] base: Base,
        id: usize,
        ts: u64,
    },
}

impl Event {
    #[inline]
    pub fn is_barrier(&self) -> bool {
        match self {
            Event::Barrier => true,
            _ => false,
        }
    }
}
