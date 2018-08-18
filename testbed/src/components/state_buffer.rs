use std::{
    time::Instant,
    collections::VecDeque,
};
use specs::prelude::*;
use nalgebra::{
    Point2,
    //Vector2,
    UnitComplex,
    wrap,
};
use crate::{
    util::*,
    prot::EntityState,
};

struct State {
    time: Instant,
    position: Point2<f32>,
    rotation: UnitComplex<f32>,
    //velocity: Vector2<f32>,
}

impl State {
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
    buf: VecDeque<State>,
}

impl StateBuffer {
    pub fn new() -> Self {
        Self { buf: VecDeque::new() }
    }

    /// Drop older positions.
    pub fn drop_older(&mut self, than: Instant) {
        while self.buf.len() >= 2 && self.buf[1].time <= than {
            self.buf.pop_front();
        }
    }

    pub fn push_state(&mut self, time: Instant, state: &EntityState) {
        self.buf.push_back(State {
            time,
            position: state.position,
            //velocity: state.velocity,
            rotation: UnitComplex::from_angle(state.rotation),
        });
    }

    pub fn interpolate(&mut self, time: Instant) -> Option<(Point2<f32>, UnitComplex<f32>)> {
        self.drop_older(time);

        // Find the two authoritative positions surrounding the rendering time.
        // Interpolate between the two surrounding authoritative positions.
        if self.buf.len() >= 2 {
            let (a, b) = (&self.buf[0], &self.buf[1]);
            if a.time <= time && time <= b.time {
                let position = State::interpolate_linear(a, b, time);
                let rotation = State::interpolate_angular(a, b, time);
                return Some((position, rotation));
            }
        }
        None
    }
}
