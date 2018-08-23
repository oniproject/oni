use nalgebra::{
    Point2,
    Translation2,
    Isometry2,
    distance_squared,
};

use crate::{
    consts::*,
    util::View,
};

use super::Boid;

pub struct Target {
    pub position: Point2<f32>,
    pub radius: f32,
}

impl Target {
    pub fn new(x: f32, y: f32, radius: f32) -> Self {
        Self {
            position: Point2::new(x, y),
            radius,
        }
    }
}

pub struct PathFollowing {
    pub path: Vec<Target>,
    pub current: usize,
}

impl PathFollowing {
    pub fn new(path: Vec<Target>) -> Self {
        Self { path, current: 0 }
    }
    pub fn target<B: Boid>(&mut self, boid: &B) -> Option<Point2<f32>> {
        let target = self.path.get(self.current)?;
        let radius2 = target.radius.powi(2);
        if radius2 > distance_squared(&boid.position(), &target.position) {
            self.current += 1;
            self.current %= self.path.len();
        }
        Some(target.position)
    }

    pub fn debug_draw(&mut self, mut view: View) {
        view.curve_loop(MAROON.into(), self.path.iter().map(|t| t.position));

        for (i, target) in self.path.iter().enumerate() {
            let t = Translation2::from_vector(target.position.coords);
            let iso = Isometry2::identity() * t;
            let r = target.radius;
            view.circ(iso, r, MAROON.into());
            if i == self.current {
                view.x(iso, r, r, RED.into());
            }
        }
    }
}
