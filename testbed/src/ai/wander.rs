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
};

use crate::{
    util::View,
};

use super::{Steering, Boid};

pub struct Wander {
    pub distance: f32,
    pub radius: f32,
    pub angle_change: Uniform<f32>,
    pub angle: f32,

    rng: SmallRng,
}

impl Wander {
    pub fn new() -> Self {
        Self {
            distance: 0.5,
            radius: 0.25,
            angle_change: Uniform::new(-0.1, 0.1),
            angle: 0.0,

            rng: SmallRng::from_entropy(),
        }
    }

    fn update_angle(&mut self) {
        self.angle += self.rng.sample(&self.angle_change);
    }

    fn circle_space<B: Boid>(&self, boid: &B) -> Isometry2<f32> {
        let velocity = boid.velocity().normalize() * self.distance;
        let velocity = Translation2::from_vector(velocity);
        boid.translation() * velocity * boid.rotation()
    }

    pub fn debug_draw<B: Boid>(&mut self, mut view: View, boid: &B) {
        let circle = self.circle_space(boid);
        let angle = UnitComplex::from_angle(self.angle);

        let color = Color::new(0.0, 0.0, 0.0);
        view.circ(circle, self.radius, color);
        view.ray(circle * angle, self.radius, color);
    }
}

impl<B: Boid> Steering<B> for Wander {
    fn steering(&mut self, boid: &B) -> Isometry2<f32> {
        self.update_angle();

        let circle = self.circle_space(boid);
        let angle = UnitComplex::from_angle(self.angle);

        let acc = (circle * angle).transform_vector(&Vector2::identity());
        let acc = Translation2::from_vector(acc);
        Isometry2::from_parts(acc, UnitComplex::identity())
    }
}
