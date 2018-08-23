use nalgebra::{
    Point2,
    Translation2,
    Isometry2,
    UnitComplex,
};

use super::{Steering, Boid};

pub struct Arrival {
    pub target: Point2<f32>,
    pub slowing_radius: f32,
    pub slowing_factor: f32,
}

impl Arrival {
    pub fn new(target: Point2<f32>) -> Self {
        Self {
            target,
            slowing_radius: 0.75,
            slowing_factor: 2.0,
        }
    }
}

impl<B: Boid> Steering<B> for Arrival {
    fn steering(&mut self, boid: &B) -> Isometry2<f32> {
        let acc = boid.max_linear_acceleration();
        let desired_velocity = self.target - boid.position();
        let distance = (desired_velocity.x * desired_velocity.y).sqrt();

        let mut desired_velocity = desired_velocity.normalize() * acc;
        if distance < self.slowing_radius {
            desired_velocity *= distance / self.slowing_factor;
        }
        let acc = desired_velocity - boid.velocity();
        let acc = Translation2::from_vector(acc);
        Isometry2::from_parts(acc, UnitComplex::identity())
    }
}
