use time::precise_time_ns;
use std::mem::{replace, zeroed};
use crate::{
    local::LOCAL,
    trace::Args,
};

pub struct ScopeComplete {
    name: &'static str,
    args: Args,
    start: u64,
}

impl ScopeComplete {
    #[inline]
    pub fn new(name: &'static str, args: Args) -> Self {
        let start = precise_time_ns();
        Self { name, args, start }
    }
    #[inline]
    pub fn with_empty_args(name: &'static str) -> Self {
        Self::new(name, Args::Empty)
    }
}

impl Drop for ScopeComplete {
    /// When the Scope is dropped it records the
    /// length of time it was alive for and records it
    /// against the Profiler.
    fn drop(&mut self) {
        let end = precise_time_ns();
        let start = self.start;
        let name = self.name;
        let args = unsafe {
            replace(&mut self.args, zeroed())
        };

        LOCAL.with(|profiler| match *profiler.borrow() {
            Some(ref profiler) => profiler.complete(start, end, name.into(), None, args),
            None => println!("ERROR: ProfileScope {} on unregistered thread!", name),
        });
    }
}
