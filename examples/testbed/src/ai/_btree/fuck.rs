
#[derive(Clone, Copy)]
enum State {
    Init,
    Running,
    Completed(bool),
}

impl State {
    fn is_done(&self) -> bool {
        match self {
            State::Completed(_) => true,
            _ => false,
        }
    }
}

pub struct Node<'a, T> {
    state: State,
    inner: Box<Inner<T> + 'a>,
}

impl<'a, T> Node<'a, T> {
    pub fn new<I>(inner: I) -> Self
        where I: Inner<T> + 'a
    {
        Self {
            state: State::Init,
            inner: Box::new(inner),
        }
    }

    pub fn tick(&mut self, world: &mut T) -> State {
        if self.state.is_done() {
            self.reset();
        }
        self.state = (*self.inner).tick(world);
        self.state
    }

    pub fn reset(&mut self) {
        if self.state == State::Init {
            return;
        }
        self.state = State::Init;
        (*self.inner).reset();
    }

    pub fn children(&self) -> Vec<Node<'a, T>> {
        (*self.inner).children()
    }

    pub fn state(&self) { self.state }
}

pub trait Inner<T> {
    fn tick(&mut self, world: &mut T) -> State;
    fn reset(&mut self);
    fn children(&self) -> Vec<&Node<T>> {
        Vec::new()
    }
}

mod status;

use self::status::Status;

trait Task {
    type State;
    fn resume(&mut self) -> Status;
}

pub enum State<'a> {
    Branch {
        current: usize,
    },
}

use std::time::{Instant, Duration};
use std::ops::{Generator, GeneratorState};

fn wait(timeout: Duration) -> impl Generator {
    let start_time = Instant::now();
    move || {
        while start_time.elapsed() < timeout {
            yield;
        }
        true
    }
}

fn sequence(children: &'static mut [impl Generator<Return=bool>]) -> impl Generator {
    move || {
        for child in children {
            match unsafe { child.resume() } {
                GeneratorState::Complete(true) => (),
                GeneratorState::Complete(false) => return false,
                GeneratorState::Yielded(_) => yield,
            }
        }
        true
    }
}

fn selector(children: &'static mut [impl Generator<Return=bool>]) -> impl Generator {
    move || {
        for child in children {
            match unsafe { child.resume() } {
                GeneratorState::Complete(true) => return true,
                GeneratorState::Complete(false) => (),
                GeneratorState::Yielded(_) => yield,
            }
        }
        true
    }
}

/*
struct Sequence {
    children: Vec<Task>,
    current: usize,
}

impl Sequence {
    fn run(childern: &[Task]) -> Status {
        for child in self.children.iter_mut() {
            match child.run() {
                Status::Success => (),
                Status::Failure => return Status::Failure,
                Status::Pending => return Status::Pending,
            }
        }
        Status::Success
    }
}
*/
