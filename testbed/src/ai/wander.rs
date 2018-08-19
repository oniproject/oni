use rand::{
    FromEntropy, Rng,
    distributions::{Distribution, Uniform},
    rngs::SmallRng,
};

use alga::linear::Transformation;
use nalgebra::{
    Point2, Vector2,
    Isometry2, Translation2,
    UnitComplex,
    Point3 as Color,
    norm, zero,
};

use crate::{
    util::View,
    components::Actor,
};

use super::Steering;

pub struct Wander {
    distance: f32,
    radius: f32,
    angle_change: Uniform<f32>,
    wander_angle: f32,

    rng: SmallRng,
}

impl Wander {
    pub fn new() -> Self {
        Self {
            distance: 0.5,
            radius: 0.25,
            angle_change: Uniform::new(-0.5, 0.5),
            wander_angle: 0.0,

            rng: SmallRng::from_entropy(),
        }
    }

    fn circle_space(&self, boid: &Actor) -> Isometry2<f32> {
        let velocity = boid.velocity.normalize() * self.distance;
        let velocity = Translation2::from_vector(velocity);
        boid.translation() * velocity * boid.rotation
    }

    pub fn debug_draw(&mut self, mut view: View, boid: &Actor) {
        let circle = self.circle_space(boid);
        let angle = UnitComplex::from_angle(self.wander_angle);

        let color = Color::new(0.0, 0.0, 0.0);
        view.circ(circle, self.radius, color);
        view.ray(circle * angle, self.radius, color);
    }
}

impl Steering for Wander {
    fn steering(&mut self, boid: &Actor) -> Isometry2<f32> {
        self.wander_angle += self.rng.sample(&self.angle_change);

        let circle = self.circle_space(boid);
        let angle = UnitComplex::from_angle(self.wander_angle);

        let acc = (circle * angle).transform_vector(&Vector2::identity());
        let acc = Translation2::from_vector(acc);
        Isometry2::from_parts(acc, UnitComplex::identity())
    }
}
