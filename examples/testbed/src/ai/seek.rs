use nalgebra::{
    Point2,
    Translation2,
    Isometry2,
    UnitComplex,
};

use super::{Steering, Boid};

pub struct Seek {
    pub target: Point2<f32>
}

impl Seek {
    pub fn new(target: Point2<f32>) -> Self {
        Self { target }
    }
}

impl<B: Boid> Steering<B> for Seek {
    fn steering(&mut self, boid: &B) -> Isometry2<f32> {
        let acc = boid.max_linear_acceleration();
        let desired_velocity = (self.target - boid.position()).normalize() * acc;
        let acc = desired_velocity - boid.velocity();
        let acc = Translation2::from_vector(acc);
        Isometry2::from_parts(acc, UnitComplex::identity())
    }
}
