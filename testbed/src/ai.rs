#![allow(dead_code, unused_imports)]

use nalgebra::{
    Point2, Vector2,
    Translation2,
    Rotation2,
    Isometry2,
    UnitComplex,
    distance,
    distance_squared,
    clamp,
};

use std::time::{Duration, Instant};

pub trait Integrator {
    fn integrate(position: Point2<f32>, velocity: Vector2<f32>) -> Point2<f32>;
}

pub struct Euler;

impl Integrator for Euler {
    fn integrate(position: Point2<f32>, velocity: Vector2<f32>) -> Point2<f32> {
        position + velocity
    }
}

/*
struct Boid {
    max_velocity: f32,
    position: Point2<f32>,
}

fn flee() {
    let desired_velocity = (position - target).normalize() * max_velocity;
    let steering = desired_velocity - velocity;
}

    let steering = truncate (steering, max_force)
    let steering = steering / mass

    let velocity = truncate (velocity + steering , max_speed)
    let position = position + velocity
*/

pub struct Boid {
    position: Point2<f32>,
    velocity: Point2<f32>,

    max_force: f32,
    max_velocity: f32,
    mass: f32,

    max_linear_acceleration: f32,

    path: Vec<Point2<f32>>,
    path_radius: f32,
    current: usize,
}

impl Boid {
    pub fn path_following(&mut self) -> Option<Point2<f32>> {
        if let Some(target) = self.path.get(self.current) {
            let r2 = self.path_radius.powi(2);
            if r2 > distance_squared(&self.position, target) {
                self.current += 1;
                self.current %= self.path.len();
            }
            Some(*target)
            //Some(seek(target))
        } else {
            None
        }
    }
}

pub struct AI {
    max_linear_acceleration: f32,
    max_speed: f32,
    max_force: f32,
    mass: f32,

    pub velocity: Vector2<f32>,
    pub rotation: UnitComplex<f32>,
    pub path: PathFollowing,
}

impl AI {
    pub fn new() -> Self {
        let path_radius = 0.5;
        Self {
            max_linear_acceleration: 1.0,
            max_speed: 1.0,
            max_force: 1.8,
            mass: 20.0,

            velocity: Vector2::new(0.0, 0.0),
            rotation: UnitComplex::new(0.0),

            path: PathFollowing::new(vec![
                Target::new(-1.0, -1.5, path_radius),
                Target::new( 3.0,  0.0, path_radius),
                Target::new(-1.0,  1.5, path_radius),
            ]),
        }
    }

    fn steering_seek(&mut self, position: Point2<f32>, target: Point2<f32>) -> Vector2<f32> {
        let acc = self.max_linear_acceleration;
        let desired_velocity = (target - position).normalize() * acc;
        desired_velocity - self.velocity
    }

    fn steering_flee(&mut self, position: Point2<f32>, target: Point2<f32>) -> Vector2<f32> {
        let acc = self.max_linear_acceleration;
        let desired_velocity = (position - target).normalize() * acc;
        desired_velocity - self.velocity
    }
}

impl crate::client::Controller for AI {
    fn run(&mut self, position: Point2<f32>) -> Option<Isometry2<f32>> {
        self.velocity = self.path.target(&position)
            .map(|target| self.steering_seek(position, target))
            .map(|steering| {
                let steering = truncate(steering, self.max_force);
                let steering = steering / self.mass;
                let velocity = truncate(self.velocity + steering, self.max_speed);
                velocity
            })
            .unwrap_or(self.velocity);

        self.rotation = UnitComplex::from_cos_sin_unchecked(
            self.velocity.x,
            self.velocity.y,
        );

        let translation = Translation2::from_vector(self.velocity);
        Some(Isometry2::from_parts(translation, self.rotation))
    }
}

fn truncate(v: Vector2<f32>, max: f32) -> Vector2<f32> {
    let len = (v.x * v.y).sqrt();
    if len == 0.0 {
        return Vector2::new(0.0, 0.0);
    }

    let i = max / len;
    v * if i < 1.0 { i } else { 1.0 }
}

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
    pub fn target(&mut self, position: &Point2<f32>) -> Option<Point2<f32>> {
        let target = self.path.get(self.current)?;
        let radius2 = target.radius.powi(2);
        if radius2 > distance_squared(position, &target.position) {
            self.current += 1;
            self.current %= self.path.len();
        }
        Some(target.position)
    }
}

pub fn seek(position: &Point2<f32>, target: &Point2<f32>) -> Vector2<f32> {
    (target - position).normalize()
}

pub fn flee(position: &Point2<f32>, target: &Point2<f32>) -> Vector2<f32> {
    (position - target).normalize()
}

pub struct Two {
    pub linear: f32,
    pub angular: f32,
}

pub trait Limiter {
    fn threshold(&self) -> Two;
    fn max_speed(&self) -> Two;
    fn max_acceleration(&self) -> Two;
}

pub trait LimiterMut {
    fn set_threshold(&mut self, threshold: Two);
    fn set_max_speed(&mut self, linear_speed: Two);
    fn set_max_acceleration(&mut self, linear_acceleration: Two);
}
