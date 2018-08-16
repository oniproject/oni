//! Utilities for working with time.

use std::time::{Duration, Instant};

const NANOS_PER_SEC: u64 = 1_000_000_000;

/// Converts a Duration to the time in seconds in an `f32`.
pub fn duration_to_secs_f32(duration: Duration) -> f32 {
    duration.as_secs() as f32 + (duration.subsec_nanos() as f32 / 1.0e9)
}

/// Converts a Duration to the time in seconds in an `f64`.
pub fn duration_to_secs_f64(duration: Duration) -> f64 {
    duration.as_secs() as f64 + (duration.subsec_nanos() as f64 / 1.0e9)
}

/// Converts a time in seconds in an `f32` to a duration.
pub fn secs_to_duration_f32(secs: f32) -> Duration {
    Duration::new(secs as u64, ((secs % 1.0) * 1.0e9) as u32)
}

/// Converts a time in seconds in an `f64` to a duration.
pub fn secs_to_duration_f64(secs: f64) -> Duration {
    Duration::new(secs as u64, ((secs % 1.0) * 1.0e9) as u32)
}

/// Converts a Duration to nanoseconds.
pub fn duration_to_nanos(duration: Duration) -> u64 {
    (duration.as_secs() * NANOS_PER_SEC) + duration.subsec_nanos() as u64
}

/// Converts nanoseconds to a Duration.
pub fn nanos_to_duration(nanos: u64) -> Duration {
    Duration::new(nanos / NANOS_PER_SEC, (nanos % NANOS_PER_SEC) as u32)
}

/// This should only be called by the engine.
/// Bad things might happen if you call this in your game.
pub trait Advance {
    /// Sets both `delta_seconds` and `delta_time` based on the seconds given.
    fn advance_seconds(&mut self, secs: f32);

    /// Sets both `delta_time` and `delta_seconds` based on the duration given.
    fn advance_time(&mut self, time: Duration);

    /// Increments the current frame number by 1.
    fn increment_frame_number(&mut self);

    /// Indicates a fixed update just finished.
    fn advance_fixed(&mut self);
}

/// Frame timing values.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Time {
    delta: (f32, Duration),
    fixed: (f32, Duration),
    last_fixed_update: Instant,
    frame_number: u64,
    absolute_time: Duration,
}

impl Time {
    /// Gets the time difference between frames in seconds.
    pub fn delta_seconds(&self) -> f32 {
        self.delta.0
    }

    /// Gets the time difference between frames.
    pub fn delta_time(&self) -> Duration {
        self.delta.1
    }

    /// Gets the fixed time step in seconds.
    pub fn fixed_seconds(&self) -> f32 {
        self.fixed.0
    }

    /// Gets the fixed time step.
    pub fn fixed_time(&self) -> Duration {
        self.fixed.1
    }

    /// Gets the current frame number.
    /// This increments by 1 every frame.
    /// There is no frame 0.
    pub fn frame_number(&self) -> u64 {
        self.frame_number
    }

    /// Gets the time at which the last fixed update was called.
    pub fn last_fixed_update(&self) -> Instant {
        self.last_fixed_update
    }

    /// Gets the time since the start of the game as seconds.
    pub fn absolute_time_seconds(&self) -> f64 {
        duration_to_secs_f64(self.absolute_time)
    }

    /// Sets both `fixed_seconds` and `fixed_time` based on the seconds given.
    pub fn set_fixed_seconds(&mut self, secs: f32) {
        self.fixed = (secs, secs_to_duration_f32(secs));
    }

    /// Sets both `fixed_time` and `fixed_seconds` based on the duration given.
    pub fn set_fixed_time(&mut self, time: Duration) {
        self.fixed = (duration_to_secs_f32(time), time);
    }
}

impl Advance for Time {
    fn advance_seconds(&mut self, secs: f32) {
        self.delta.0 = secs;
        self.delta.1 = secs_to_duration_f32(secs);
        self.absolute_time += self.delta.1;
    }
    fn advance_time(&mut self, time: Duration) {
        self.delta.0 = duration_to_secs_f32(time);
        self.delta.1 = time;
        self.absolute_time += self.delta.1;
    }
    fn increment_frame_number(&mut self) {
        self.frame_number += 1;
    }
    fn advance_fixed(&mut self) {
        self.last_fixed_update += self.fixed.1;
    }
}

impl Default for Time {
    fn default() -> Time {
        Time {
            delta: (0.0, Duration::from_secs(0)),
            fixed: (duration_to_secs_f32(Duration::new(0, 16666666)), Duration::new(0, 16666666)),
            last_fixed_update: Instant::now(),
            frame_number: 0,
            absolute_time: Duration::default(),
        }
    }
}

/// A stopwatch which accurately measures elapsed time.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Stopwatch {
    /// Initial state with an elapsed time value of 0 seconds.
    Waiting,
    /// Stopwatch has started counting the elapsed time since this `Instant`
    /// and accumuluated time from previous start/stop cycles `Duration`.
    Started(Duration, Instant),
    /// Stopwatch has been stopped and reports the elapsed time `Duration`.
    Ended(Duration),
}

impl Default for Stopwatch {
    fn default() -> Self {
        Stopwatch::Waiting
    }
}

impl Stopwatch {
    /// Creates a new stopwatch.
    pub fn new() -> Self {
        Default::default()
    }

    /// Retrieves the elapsed time.
    pub fn elapsed(&self) -> Duration {
        match *self {
            Stopwatch::Waiting => Duration::new(0, 0),
            Stopwatch::Started(dur, start) => dur + start.elapsed(),
            Stopwatch::Ended(dur) => dur,
        }
    }

    /// Stops, resets, and starts the stopwatch again.
    pub fn restart(&mut self) {
        *self = Stopwatch::Started(Duration::new(0, 0), Instant::now());
    }

    /// Starts, or resumes, measuring elapsed time. If the stopwatch has been
    /// started and stopped before, the new results are compounded onto the
    /// existing elapsed time value.
    ///
    /// Note: Starting an already running stopwatch will do nothing.
    pub fn start(&mut self) {
        match *self {
            Stopwatch::Waiting => self.restart(),
            Stopwatch::Ended(dur) => {
                *self = Stopwatch::Started(dur, Instant::now());
            }
            _ => {}
        }
    }

    /// Stops measuring elapsed time.
    ///
    /// Note: Stopping a stopwatch that isn't running will do nothing.
    pub fn stop(&mut self) {
        if let Stopwatch::Started(dur, start) = *self {
            *self = Stopwatch::Ended(dur + start.elapsed());
        }
    }

    /// Clears the current elapsed time value.
    pub fn reset(&mut self) {
        *self = Stopwatch::Waiting;
    }
}

#[test]
fn elapsed() {
    const DURATION: u64 = 1; // in seconds.
    const UNCERTAINTY: u32 = 10; // in percents.
    let mut watch = Stopwatch::new();

    watch.start();
    std::thread::sleep(Duration::from_secs(DURATION));
    watch.stop();

    // check that elapsed time was DURATION sec +/- UNCERTAINTY%
    let elapsed = watch.elapsed();
    let duration = Duration::new(DURATION, 0);
    let lower = duration / 100 * (100 - UNCERTAINTY);
    let upper = duration / 100 * (100 + UNCERTAINTY);
    assert!(
        elapsed < upper && elapsed > lower,
        "expected {} +- {}% seconds, got {:?}",
        DURATION,
        UNCERTAINTY,
        elapsed
    );
}

#[test]
fn reset() {
    const DURATION: u64 = 2; // in seconds.
    let mut watch = Stopwatch::new();

    watch.start();
    std::thread::sleep(Duration::from_secs(DURATION));
    watch.stop();
    watch.reset();

    assert_eq!(0, watch.elapsed().subsec_nanos());
}

#[test]
fn restart() {
    const DURATION0: u64 = 2; // in seconds.
    const DURATION: u64 = 1; // in seconds.
    const UNCERTAINTY: u32 = 10; // in percents.
    let mut watch = Stopwatch::new();

    watch.start();
    std::thread::sleep(Duration::from_secs(DURATION0));
    watch.stop();

    watch.restart();
    std::thread::sleep(Duration::from_secs(DURATION));
    watch.stop();

    // check that elapsed time was DURATION sec +/- UNCERTAINTY%
    let elapsed = watch.elapsed();
    let duration = Duration::new(DURATION, 0);
    let lower = duration / 100 * (100 - UNCERTAINTY);
    let upper = duration / 100 * (100 + UNCERTAINTY);
    assert!(
        elapsed < upper && elapsed > lower,
        "expected {} +- {}% seconds, got {:?}",
        DURATION,
        UNCERTAINTY,
        elapsed
    );
}

// test that multiple start-stop cycles are cumulative
#[test]
fn stop_start() {
    const DURATION: u64 = 3; // in seconds.
    const UNCERTAINTY: u32 = 10; // in percents.
    let mut watch = Stopwatch::new();

    for _ in 0..DURATION {
        watch.start();
        std::thread::sleep(Duration::from_secs(1));
        watch.stop();
    }

    // check that elapsed time was DURATION sec +/- UNCERTAINTY%
    let elapsed = watch.elapsed();
    let duration = Duration::new(DURATION, 0);
    let lower = duration / 100 * (100 - UNCERTAINTY);
    let upper = duration / 100 * (100 + UNCERTAINTY);
    assert!(
        elapsed < upper && elapsed > lower,
        "expected {}  +- {}% seconds, got {:?}",
        DURATION,
        UNCERTAINTY,
        elapsed
    );
}
