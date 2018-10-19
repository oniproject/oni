use super::{
    forest::{World, Tree},
    //status::Status::{self, Success, Failure, Pending},
};

/*
fn do_sun(world: &mut World) -> Status {
    let can = world.can_shine();
    if can { world.toggle_sun(); }
    can.into()
}

fn do_rain(world: &mut World) -> Status {
    let can = world.can_rain();
    if can { world.rain(); }
    can.into()
}

fn world_behavior(states: &mut [usize], world: &mut World) -> Status {
    let children = &[do_sun, do_rain];

    let current = &mut states[0];
    while let Some(child) = children.get(*current) {
        match child(world) {
            Failure => *current += 1,
            status => return status,
        }
    }
    Failure
}
*/


/// Describes a behavior.
///
/// This is used for more complex event logic.
/// Can also be used for game AI.
#[derive(Clone, Deserialize, Serialize, PartialEq)]
pub enum Behavior<A> {
    /// Waits an amount of time before continuing.
    ///
    /// f64: Time in seconds
    Wait(f64),
    /// Wait forever.
    WaitForever,
    /// A high level description of an action.
    Action(A),
    /// Converts `Success` into `Failure` and vice versa.
    Fail(Box<Behavior<A>>),
    /// Ignores failures and returns `Success`.
    AlwaysSucceed(Box<Behavior<A>>),
    /// Runs behaviors one by one until a behavior succeeds.
    ///
    /// If a behavior fails it will try the next one.
    /// Fails if the last behavior fails.
    /// Can be thought of as a short-circuited logical OR gate.
    Select(Vec<Behavior<A>>),
    /// `If(condition, success, failure)`
    If(Box<Behavior<A>>, Box<Behavior<A>>, Box<Behavior<A>>),
    /// Runs behaviors one by one until all succeeded.
    ///
    /// The sequence fails if a behavior fails.
    /// The sequence succeeds if all the behavior succeeds.
    /// Can be thought of as a short-circuited logical AND gate.
    Sequence(Vec<Behavior<A>>),
    /// Loops while conditional behavior is running.
    ///
    /// Succeeds if the conditional behavior succeeds.
    /// Fails if the conditional behavior fails,
    /// or if any behavior in the loop body fails.
    While(Box<Behavior<A>>, Vec<Behavior<A>>),
    /// Runs all behaviors in parallel until all succeeded.
    ///
    /// Succeeds if all behaviors succeed.
    /// Fails is any behavior fails.
    WhenAll(Vec<Behavior<A>>),
    /// Runs all behaviors in parallel until one succeeds.
    ///
    /// Succeeds if one behavior succeeds.
    /// Fails if all behaviors failed.
    WhenAny(Vec<Behavior<A>>),
    /// Runs all behaviors in parallel until all succeeds in sequence.
    ///
    /// Succeeds if all behaviors succeed, but only if succeeding in sequence.
    /// Fails if one behavior fails.
    After(Vec<Behavior<A>>),
}
