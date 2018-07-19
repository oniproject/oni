use std::{
    time::{Instant, Duration},
    thread::sleep,
};

pub struct EventLoop {
    quit: bool,
    dt_update: Duration,
    last_update: Instant,
}

impl EventLoop {
    pub fn new(dt_update: Duration) -> Self {
        Self {
            dt_update,
            quit: false,
            last_update: Instant::now(),
        }
    }

    pub fn quit(&mut self) {
        self.quit = true;
    }
}

impl Iterator for EventLoop {
    type Item = ();
    fn next(&mut self) -> Option<()> {
        let current_time = Instant::now();
        let next_time = self.last_update + self.dt_update;
        if next_time > current_time {
            sleep(next_time - current_time);
            self.last_update += self.dt_update;
        }

        if self.quit {
            None
        } else {
            Some(())
        }
    }
}
