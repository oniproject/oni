use std::f64;

use super::{
    Action,
    After,
    AlwaysSucceed,
    Behavior,
    Failure,
    If,
    Invert,
    Running,
    Select,
    Sequence,
    Status,
    Success,
    Wait,
    WaitForever,
    WhenAll,
    WhenAny,
    While,
};

/// The action is still running.
pub const RUNNING: (Status, f64) = (Running, 0.0);

/// The arguments in the action callback.
pub struct ActionArgs<'a, A: 'a, S: 'a> {
    /// The remaining delta time.
    pub dt: f64,
    /// The action running.
    pub action: &'a A,
    /// The state of the running action, if any.
    pub state: &'a mut Option<S>,
}

/// Keeps track of a behavior.
#[derive(Clone, Deserialize, Serialize, PartialEq)]
pub enum State<A, S> {
    /// Executes an action.
    Action(A, Option<S>),
    /// Converts `Success` into `Failure` and vice versa.
    Invert(Box<State<A, S>>),

    /// Ignores failures and always return `Success`.
    AlwaysSucceed(Box<State<A, S>>),

    /// Keeps track of waiting for a period of time before continuing.
    ///
    /// f64: Total time in seconds to wait
    ///
    /// f64: Time elapsed in seconds
    Wait(f64, f64),

    /// Waits forever.
    WaitForever,

    /// Keeps track of an `If` behavior.
    /// If status is `Running`, then it evaluates the condition.
    /// If status is `Success`, then it evaluates the success behavior.
    /// If status is `Failure`, then it evaluates the failure behavior.
    If(Box<Behavior<A>>, Box<Behavior<A>>, Status, Box<State<A, S>>),

    /// Keeps track of a `Select` behavior.
    Select(Vec<Behavior<A>>, usize, Box<State<A, S>>),
    /// Keeps track of an `Sequence` behavior.
    Sequence(Vec<Behavior<A>>, usize, Box<State<A, S>>),

    /// Keeps track of a `While` behavior.
    While(Box<State<A, S>>, Vec<Behavior<A>>, usize, Box<State<A, S>>),

    /// Keeps track of a `WhenAll` behavior.
    WhenAll(Vec<Option<State<A, S>>>),
    /// Keeps track of a `WhenAny` behavior.
    WhenAny(Vec<Option<State<A, S>>>),

    /// Keeps track of an `After` behavior.
    After(usize, Vec<State<A, S>>),
}

impl<A: Clone, S> State<A, S> {
    /// Creates a state from a behavior.
    pub fn new(behavior: Behavior<A>) -> Self {
        match behavior {
            Action(action) => State::Action(action, None),
            Invert(ev) => State::Invert(Box::new(State::new(*ev))),
            AlwaysSucceed(ev) => State::AlwaysSucceed(Box::new(State::new(*ev))),
            Wait(dt) => State::Wait(dt, 0.0),
            WaitForever => State::WaitForever,
            If(condition, success, failure) => {
                let state = State::new(*condition);
                State::If(success, failure, Running, Box::new(state))
            }
            Select(sel) => {
                let state = State::new(sel[0].clone());
                State::Select(sel, 0, Box::new(state))
            }
            Sequence(seq) => {
                let state = State::new(seq[0].clone());
                State::Sequence(seq, 0, Box::new(state))
            }
            While(ev, rep) => {
                let state = State::new(rep[0].clone());
                State::While(Box::new(State::new(*ev)), rep, 0, Box::new(state))
            }
            WhenAll(all) => State::WhenAll(all.into_iter().map(
                    |ev| Some(State::new(ev))).collect()),
            WhenAny(all) => State::WhenAny(all.into_iter().map(
                    |ev| Some(State::new(ev))).collect()),
            After(seq) => State::After(0, seq.into_iter().map(
                    |ev| State::new(ev)).collect()),
        }
    }

    /// Updates the cursor that tracks an event.
    ///
    /// The action need to return status and remaining delta time.
    /// Returns status and the remaining delta time.
    ///
    /// Passes event, delta time in seconds, action and state to closure.
    /// The closure should return a status and remaining delta time.
    pub fn event<F>(&mut self, delta: f64, callback: &mut F) -> (Status, f64)
        where F: FnMut(ActionArgs<A, S>) -> (Status, f64)
    {
        match self {
            State::Action(action, state) => {
                // Execute action.
                callback(ActionArgs { dt: delta, action, state })
            }
            State::Invert(cur) => {
                match cur.event(delta, callback) {
                    (Running, dt) => (Running, dt),
                    (Failure, dt) => (Success, dt),
                    (Success, dt) => (Failure, dt),
                }
            }
            State::AlwaysSucceed(cur) => {
                match cur.event(delta, callback) {
                    (Running, dt) => (Running, dt),
                    (_, dt) => (Success, dt),
                }
            }
            State::Wait(wait_t, t) => wait(delta, *wait_t, t),
            State::If(success, failure, status, state) => {
                // Run in a loop to evaluate success or failure with
                // remaining delta time after condition.
                loop {
                    *status = match *status {
                        Running => {
                            match state.event(delta, callback) {
                                (Running, dt) => return (Running, dt),
                                (Success, _) => {
                                    **state = State::new((**success).clone());
                                    Success
                                }
                                (Failure, _) => {
                                    **state = State::new((**failure).clone());
                                    Failure
                                }
                            }
                        }
                        _ => return state.event(delta, callback),
                    }
                }
            }
            State::Select(seq, i, cursor) =>
                sequence(true, delta, seq, i, cursor, callback),
            State::Sequence(seq, i, cursor) =>
                sequence(false, delta, seq, i, cursor, callback),
            State::While(ev_cursor, rep, i, cursor) =>
                _while(delta, ev_cursor, rep, i, cursor, callback),
            State::WhenAll(cursors) => when_all(false, delta, cursors, callback),
            State::WhenAny(cursors) => when_all(true , delta, cursors, callback),
            State::After(i, cursors) => after(delta, i, cursors, callback),
            State::WaitForever => RUNNING,
        }
    }
}

fn wait(delta: f64, wait_t: f64, t: &mut f64) -> (Status, f64) {
    let target = *t + delta;
    if target >= wait_t {
        let remaining_dt = target - wait_t;
        *t = wait_t;
        (Success, remaining_dt)
    } else {
        *t += delta;
        RUNNING
    }
}

// `Sequence` and `Select` share same algorithm.
//
// `Sequence` fails if any fails and succeeds when all succeeds.
// `Select` succeeds if any succeeds and fails when all fails.
fn sequence<A: Clone, S, F>(
    select: bool,
    mut delta: f64,
    seq: &Vec<Behavior<A>>,
    i: &mut usize,
    cursor: &mut Box<State<A, S>>,
    callback: &mut F
) -> (Status, f64)
    where F: FnMut(ActionArgs<A, S>) -> (Status, f64)
{
    let (status, inv_status) = if select {
        // `Select`
        (Failure, Success)
    } else {
        // `Sequence`
        (Success, Failure)
    };
    while *i < seq.len() {
        match cursor.event(delta, callback) {
            (Running, _) => break,
            (s, new_dt) if s == inv_status => return (inv_status, new_dt),
            (s, new_dt) if s == status => delta = new_dt,
            _ => unreachable!()
        };
        *i += 1;
        // If end of sequence,
        // return the 'dt' that is left.
        if *i >= seq.len() { return (status, delta); }
        // Create a new cursor for next event.
        // Use the same pointer to avoid allocation.
        **cursor  = State::new(seq[*i].clone());
    }
    RUNNING
}

// `WhenAll` and `WhenAny` share same algorithm.
//
// `WhenAll` fails if any fails and succeeds when all succeeds.
// `WhenAny` succeeds if any succeeds and fails when all fails.
fn when_all<A: Clone, S, F>(
    any: bool,
    delta: f64,
    cursors: &mut Vec<Option<State<A, S>>>,
    callback: &mut F
) -> (Status, f64)
    where F: FnMut(ActionArgs<A, S>) -> (Status, f64)
{
    let (status, inv_status) = if any {
        // `WhenAny`
        (Failure, Success)
    } else {
        // `WhenAll`
        (Success, Failure)
    };
    // Get the least delta time left over.
    let mut min_dt = f64::MAX;
    // Count number of terminated events.
    let mut terminated = 0;
    for cur in cursors.iter_mut() {
        match *cur {
            None => (),
            Some(ref mut cur) => {
                match cur.event(delta, callback) {
                    (Running, _) => continue,

                    // Fail for `WhenAll`.
                    // Succeed for `WhenAny`.
                    (s, new_dt) if s == inv_status => return (inv_status, new_dt),
                    (s, new_dt) if s == status => min_dt = min_dt.min(new_dt),
                    _ => unreachable!()
                }
            }
        }

        terminated += 1;
        *cur = None;
    }
    match terminated {
        // If there are no events, there is a whole 'dt' left.
        0 if cursors.len() == 0 => (status, delta),
        // If all events terminated, the least delta time is left.
        n if cursors.len() == n => (status, min_dt),
        _ => RUNNING
    }
}

fn after<A: Clone, S, F>(
    delta: f64,
    i: &mut usize,
    cursors: &mut [State<A, S>],
    callback: &mut F,
) -> (Status, f64)
    where F: FnMut(ActionArgs<A, S>) -> (Status, f64)
{
    // Get the least delta time left over.
    let mut min_dt = f64::MAX;
    for j in *i..cursors.len() {
        match cursors[j].event(delta, callback) {
            (Running, _) => min_dt = 0.0,
            (Success, new_dt) => {
                // Remaining delta time must be less to succeed.
                if *i == j && new_dt < min_dt {
                    *i += 1;
                    min_dt = new_dt;
                } else {
                    // Return least delta time because
                    // that is when failure is detected.
                    return (Failure, min_dt.min(new_dt));
                }
            }
            (Failure, new_dt) => return (Failure, new_dt),
        };
    }
    if *i == cursors.len() {
        (Success, min_dt)
    } else {
        RUNNING
    }
}

fn _while<A: Clone, S, F>(
    mut delta: f64,
    ev_cursor: &mut Box<State<A, S>>,
    rep: &mut Vec<Behavior<A>>,
    i: &mut usize,
    cursor: &mut Box<State<A, S>>,
    callback: &mut F,
) -> (Status, f64)
    where F: FnMut(ActionArgs<A, S>) -> (Status, f64)
{
    // If the event terminates, do not execute the loop.
    match ev_cursor.event(delta, callback) {
        (Running, _) => (),
        x => return x,
    };
    let cur = cursor;
    loop {
        match cur.event(delta, callback) {
            (Failure, x) => return (Failure, x),
            (Running, _) => break,
            // Change update event with remaining delta time.
            (Success, new_dt) => delta = new_dt,
        };
        *i += 1;
        // If end of repeated events,
        // start over from the first one.
        if *i >= rep.len() { *i = 0; }
        // Create a new cursor for next event.
        // Use the same pointer to avoid allocation.
        **cur = State::new(rep[*i].clone());
    }
    RUNNING
}
