use super::{Status, Task};

use std::time::{Instant, Duration};

pub trait Leaf {
    fn start(&mut self) {}
    fn run(&mut self) -> Status;
}

/// Failure is a leaf that immediately fails.
pub struct Failure;

impl Leaf for Failure {
    fn run(&mut self) -> Status { Status::Failure }
}

/// Success is a leaf that immediately succeeds.
pub struct Success;
impl Leaf for Success {
    fn run(&mut self) -> Status { Status::Success }
}

/// Wait is a leaf that keeps running for the specified amount of time then succeeds.
pub struct Wait {
    start_time: Instant,
    timeout: Duration,
}

impl Wait {
    /// Creates a Wait task running for the specified number of seconds.
    pub fn new(timeout: Duration) {
        Self {
            timeout,
            start_time: Instant::now(),
        }
    }
}

impl Leaf for Wait {
    fn start(&mut self) {
        self.start_time = Instant::now();
    }
    fn run(&mut self) -> Status {
        if self.start_time.elapsed() < self.timeout {
            Status::Pending
        } else {
            Status::Success
        }
    }
}
