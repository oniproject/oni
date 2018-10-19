mod status;
mod forest;
//mod tests;
//mod tests2;
mod tests3;

/*
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Status {
    Success,
    Failure,
    Pending,
}

use self::Status::{
    Success,
    Failure,
    Pending,
};

use std::time::{Instant, Duration};

pub enum Behavior<'a, S: 'a> {
    Action(S),
    Wait(Duration),
    WaitForever,

    Fail(&'a Behavior<'a, S>),
    AlwaysSuccess(&'a Behavior<'a, S>),

    Select(&'a [&'a Behavior<'a, S>]),
    Sequence(&'a [&'a Behavior<'a, S>])

    WhenAll(&'a [&'a Behavior<'a, S>]),
    WhenAny(&'a [&'a Behavior<'a, S>]),

    After(&'a [&'a Behavior<'a, S>]),

    If(
        &'a Behavior<'a, S>,
        &'a Behavior<'a, S>,
        &'a Behavior<'a, S>,
    ),
    While(
        &'a Behavior<'a, S>,
        &'a Behavior<'a, S>,
    ),
}

pub const PENDING: (Status, Duration) = (Pending, Duration::from_secs(0));

pub struct Args<'a, E: 'a, A: 'a, S: 'a> {
    pub event: &'a E,
    pub elapsed: Duration,
    pub action: &'a A,
    pub state: &'a mut Option<S>,
}

pub enum State<A, S> {
    Action(A, Option<S>),
    Fail(Box<State<A, S>>),
    AlwaysSuccess(Box<State<A, S>>),

    Wait {
        total: Instant,
        elapsed: Duration,
    }
    WaitForever,

    WhenAny(Vec<Option<State<A, S>>>),
    WhenAll(Vec<Option<State<A, S>>>),

    After {
        current: usize,
        data: Vec<State<A, S>>,
    }
}

fn sequence_select<A, S, E, F>(
    select: bool,
    upd: Option<Duration>,
    seq: &Vec<Behavior<A>>,
    i: &mut usize,
    cursor: &mut Box<State<A, S>>,
    e: &E,
    f: &mut F
) -> (Status, Duration)
    where A: Clone,
          E: GenericEvent,
          F: FnMut(Args<E, A, S>) -> (Status, Duration)
{
    let (status, inv_status) = select {
        (Failure, Success)
    } else {
        (Success, Failure)
    };

    let mut remaining_dt = upd.unwrap_or_default();
    let mut remaining_e;

    while *i < seq.len() {
        let _e = match upd {
            Some(_) => {
                remaining_e = UpdateEvent::from_dt(remaining_dt, e).unwrap();
                &remaining_e
            }
            _ => e,
        };
        match cursor.event(_e, f) {
            (Pending, _) => break,
            (s, new_dt) if s == inv_status => return (inv_status, new_dt),
            (s, new_dt) if s == status => {
                remaining_dt = match upd {
                    Some(_) => new_dt,
                    _ => if *i == seq.len() - 1 {
                        return (status, new_dt);
                    } else {
                        *i += 1;
                        **cursor = State::new(seq[*i].clone());
                    }
                }
            }
            _ => unreachable!(),
        };
        *i += 1;
        if *i >= seq.len() { return (status, remaining_dt); }
        **cursor = State::new(seq[*i].clone());
    }
    PENDING
}

fn when_any_all<A, S, E, F>(
    any: bool,
    upd: Option<Duration>,
    cursors: &mut Vec<Option<State<A, S>>>,
    e: &E,
    f: &mut F,
) -> (Status, Duration)
    where A: Clone,
          E: GenericEvent,
          F: FnMut(Args<E, A, S>) -> (Status, Duration)
{
    let (status, inv_status) = select {
        (Failure, Success)
    } else {
        (Success, Failure)
    };

    let mut min_dt = Duration::from_nanos(u64::MAX);
    let mut terminated = 0;

    for cur in cursors.iter_mut() {
        match *cur {
            None => (),
            Some(ref mut cur) => {
                match cur.event(e, f) {
                    (Pending, _) => continue,
                    (s, new_dt) if s == inv_status => return (inv_status, new_dt);
                    (s, new_dt) if s == status => min_dt = min_dt.min(new_dt),
                    _ => unreachable!(),
                }
            }
        }
    }

    match terminated {
        0 if cursors.len() == 0 => (status, upd.unwrap_or_default()),
        n if cursors.len() == n => (status, min_dt),
        _ => PENDING,
    }
}

impl<A: Clone, S> From<Behavior<A>> for State<A, S> {
    fn from(behavior: Behavior<A>) -> Self {
        match behavior {
            Behavior::Action(action) => State::Action(action, None),
            Behavior::Fail(ev) => State::Fail(Box::new(State::from(*ev))),

            Behavior::AlwaysSuccess(ev) => State::AlwaysSuccess(Box::new(State::from(*ev))),

            Behavior::Wait(dt) => State::Wait(dt, Instant::now()),
            Behavior::If(cond, success, failure) => {
                let state = State::from(*cond);
                State::If(success, failure, Pending, Box::new(state))
            }

            Behavior::While(ev, rep) => {
                let state = State::from(rep[0].clone());
                State::While(Box::new(State::from(*ev), rep, 0, Box::new(state))
            }

            Behavior::Select(sel) => {
                let state = State::from(sel[0].clone());
                State::Select(sel, 0, Box::new(state))
            }
            Behavior::Sequence(sel) => {
                let state = State::from(sel[0].clone());
                State::Sequence(sel, 0, Box::new(state))
            }

            Behavior::WhenAll(v) =>
                State::WhenAll(v.into_iter().map(State::from).collect()),

            Behavior::WhenAny(vec) =>
                State::WhenAny(vec.into_iter().map(State::from).collect()),
        }
    }
}


*/
