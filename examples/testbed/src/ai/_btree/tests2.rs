use std::rc::Rc;
use std::cell::RefCell;

use super::forest::{World, Tree};
use super::status::Status;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Test {
    Success,
    Failure,
}

impl From<bool> for Test {
    fn from(is: bool) -> Self {
        if is { Test::Success } else { Test::Failure }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum State {
    Run,
    End,
}

impl From<bool> for State {
    fn from(is_running: bool) -> Self {
        if is_running { State::Run } else { State::End }
    }
}

pub enum Behavior<'a, T: 'a> {
    Selector(&'a [Behavior<'a, T>]),
    Sequence(&'a [Behavior<'a, T>]),
    Condition(fn(&T) -> bool, &'a Behavior<'a, T>),
    Action(fn(&mut T) -> bool),
}

impl<'a, T: 'a> Behavior<'a, T> {
    pub fn tick(&self, context: &mut T) -> Status {
        match self {
            Selector(children) => {
                for child in children.iter() {
                    match child.tick(context) {
                        Status::Failure => continue,
                        Status::Success => return Status::Success,
                        Status::Pending => return Status::Pending,
                    }
                }
                Status::Failure
            }
            Sequence(children) => {
                for child in children.iter() {
                    match child.tick(context) {
                        Status::Success => continue,
                        Status::Failure => return Status::Failure,
                        Status::Pending => return Status::Pending,
                    }
                }
                Status::Success
            }
            Condition(cond, body) => {
                if cond(context) {
                    body.tick(context)
                } else {
                    Status::Failure
                }
            }
            Action(action) => {
                if action(context) {
                    Status::Pending
                } else {
                    Status::Success
                }
            }
        }
    }
}

use self::Behavior::*;

fn world_behavior() -> Behavior<'static, World> {
    Selector(&[
        Condition(World::can_shine, &Action(World::toggle_sun)),
        Condition(World::can_rain, &Action(World::rain)),
    ])
}

fn tree_behavior() -> Behavior<'static, Tree> {
    Selector(&[
        Sequence(&[
            Condition(Tree::can_make_energy, &Action(Tree::make_energy)),
            Condition(Tree::can_grow, &Action(Tree::grow)),
            Condition(Tree::can_emit_oxygen, &Action(Tree::emit_oxygen)),
        ]),
        Condition(Tree::can_gather_sun, &Action(Tree::gather_sun)),
        Condition(Tree::can_gather_water, &Action(Tree::gather_water)),
    ])
}

#[test]
fn run() {
    let world = World::new();
    let world = Rc::new(RefCell::new(world));

    let mut tree = Tree::new(world.clone());

    loop {
        {
            tree_behavior().tick(&mut tree);
        }

        {
            let mut world = world.borrow_mut();
            world_behavior().tick(&mut world);
        }

        {
            let world = world.borrow();
            println!("  Tree: {:2?}m | {:?} water | {:?} sun",
                     tree.height, tree.water, tree.sun);

            let nd = if world.sunny { "day" } else { "night" };
            println!("  World:    | {:?} water | {:?} oxygen | {:5}",
                     world.groundwater, world.oxygen,  nd);

            println!();
        }

        use std::thread::sleep;
        use std::time::Duration;

        sleep(Duration::from_millis(500))
    }
}
