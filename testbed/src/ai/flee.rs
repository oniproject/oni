use nalgebra::{
    Point2,
    Translation2,
    Isometry2,
    UnitComplex,
};

use crate::components::Actor;
use super::Steering;

pub struct Flee {
    pub target: Point2<f32>
}

impl Flee {
    pub fn new(target: Point2<f32>) -> Self {
        Self { target }
    }
}

impl Steering for Flee {
    fn steering(&mut self, boid: &Actor) -> Isometry2<f32> {
        let acc = boid.max_linear_acceleration;
        let desired_velocity = (boid.position - self.target).normalize() * acc;
        let acc = desired_velocity - boid.velocity;
        let acc = Translation2::from_vector(acc);
        Isometry2::from_parts(acc, UnitComplex::identity())
    }
}
