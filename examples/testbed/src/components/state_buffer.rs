use std::time::Instant;
    //collections::VecDeque,
use specs::prelude::*;
use nalgebra::{
    Point2,
    //Vector2,
    Translation2,
    Isometry2,
    UnitComplex,
    wrap,
};
use crate::{
    util::*,
    prot::EntityState,
};

use super::Actor;

pub struct State {
    pub time: Instant,
    pub position: Point2<f32>,
    pub rotation: UnitComplex<f32>,
    //pub velocity: Vector2<f32>,
    pub fire: bool,
    pub damage: bool,
}

impl State {
    pub fn transform(&self) -> Isometry2<f32> {
        let pos = self.position.coords;
        let pos = Translation2::from_vector(pos);
        Isometry2::from_parts(pos, self.rotation)
    }

    fn delta(a: &Self, b: &Self, time: Instant) -> f32 {
        duration_to_secs(  time - a.time) /
        duration_to_secs(b.time - a.time)
    }

    fn interpolate_linear(a: &Self, b: &Self, time: Instant) -> Point2<f32> {
        a.position + (b.position - a.position) * Self::delta(a, b, time)
    }

    fn interpolate_angular(a: &Self, b: &Self, time: Instant) -> UnitComplex<f32> {
        use std::f32::consts::PI;
        let (from, to) = (a.rotation.angle(), b.rotation.angle());
        let angle = from + wrap(to - from, -PI, PI) * Self::delta(a, b, time);
        UnitComplex::from_angle(angle)
    }
}

#[derive(Component)]
#[storage(VecStorage)]
pub struct StateBuffer {
    buf: Vec<State>,
}

impl StateBuffer {
    pub fn new() -> Self {
        Self { buf: Vec::new() }
    }

    pub fn iter(&self) -> impl Iterator<Item=&State> {
        self.buf.iter()
    }

    /// Drop older positions.
    pub fn drop_older(&mut self, than: Instant) {
        while self.buf.len() >= 2 && self.buf[1].time <= than {
            self.buf.remove(0);
        }
    }

    pub fn push_state(&mut self, time: Instant, state: &EntityState) {
        self.buf.push(State {
            time,
            position: state.position(),
            //velocity: state.velocity,
            rotation: state.rotation(),

            fire: state.fire(),
            damage: state.damage(),
        });
    }

    pub fn interpolate_linear(&self, time: Instant) -> Option<Point2<f32>> {
        for ab in self.buf.windows(2) {
            let (a, b) = (&ab[0], &ab[1]);
            if a.time <= time && time <= b.time {
                return Some(State::interpolate_linear(a, b, time));
            }
        }
        None
    }

    pub fn interpolate(&mut self, time: Instant, actor: &mut Actor) -> bool {
        // Find the two authoritative positions surrounding the rendering time.
        // Interpolate between the two surrounding authoritative positions.
        for ab in self.buf.windows(2) {
            let (a, b) = (&ab[0], &ab[1]);
            if a.time <= time && time <= b.time {
                actor.position = State::interpolate_linear(a, b, time);
                actor.rotation = State::interpolate_angular(a, b, time);
                actor.fire = a.fire;
                actor.damage = a.damage;
                return true;
            }
        }
        false
    }
}
