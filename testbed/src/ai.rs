use nalgebra::{
    Point2, Vector2,
    Translation2,
    Rotation2,
    Isometry2,
    UnitComplex,
    distance, distance_squared,
};

use std::time::{Duration, Instant};
use crate::input::*;


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
            if distance_squared(&self.position, target) <= self.path_radius.powi(2) {
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
    /*
    pub path: Vec<Point2<f32>>,
    pub path_radius: f32,
    pub current: usize,
    */
    v: bool,
    last: Instant,
}

impl AI {
    pub fn new() -> Self {
        Self {
            v: false,
            last: Instant::now(),
        }
    }

    /*
    pub fn stick(&mut self, position: Point2<f32>) -> Option<Vector2<f32>> {
        if let Some(target) = self.path.get(self.current) {
            if distance_squared(&position, target) <= self.path_radius.powi(2) {
                self.current += 1;
                self.current %= self.path.len();
            }
            let steering = seek(&position, target, 2.0);
            Some(steering.translation.vector)
        } else {
            None
        }
    }
    */

    pub fn gen_stick(&mut self) -> Option<Stick> {
        let mut stick = Stick::default();
        let now = Instant::now();
        let sec = Duration::from_millis(1000);
        if self.last + sec <= now {
            self.last += sec;
            self.v = !self.v;
        }
        stick.x.action(true, self.v);
        Some(stick)
        //None
    }
}

pub fn seek(position: &Point2<f32>, target: &Point2<f32>, max_acc: f32) -> Isometry2<f32> {
    let delta = (target - position).normalize();
    Isometry2::from_parts(
        Translation2::from_vector(delta * max_acc),
        UnitComplex::identity(),
    )

    /*
    //let steering = desired - self.velocity;
    let x = desired.x - self.velocity.x;
    let y = desired.y - self.velocity.y;
    Vector2::new(x, y)
    */
}

/*
pub fn flee(actor: &Boid, target: Point2<f32>) -> Isometry2<f32> {
    let delta = (self.position - target).normalize();
    Isometry2::from_parts(
        Translation2::from_vector(delta * actor.max_linear_acceleration),
        UnitComplex::identity(),
    )
}
*/



/*
fn truncate(p: Point2<f32>, max: f32) -> Vector2<f32> {
    let i = max / p.distance(Point2::origin());
    p.scale_by(nalgebra::min(i, 1.0))
}
*/
