#![allow(dead_code)]

use nalgebra::{
    Point2, Vector2,
    Point3 as Color,
    Translation2,
    Isometry2,
    UnitComplex,
    norm,
    zero,
    id,
};

use crate::{
    util::View,
    components::{Controller, Actor},
};

//mod btree;

mod arrival;
mod seek;
mod path;
mod wander;

use self::seek::Seek;
use self::arrival::Arrival;
use self::path::{Target, PathFollowing};
use self::wander::Wander;

pub trait Integrator {
    fn integrate(position: Point2<f32>, velocity: Vector2<f32>) -> Point2<f32>;
}

pub struct Euler;

impl Integrator for Euler {
    fn integrate(position: Point2<f32>, velocity: Vector2<f32>) -> Point2<f32> {
        position + velocity
    }
}

pub struct AI {
    pub path: PathFollowing,
    pub wander: Wander,
}

impl AI {
    pub fn new() -> Self {
        let path_radius = 0.2;
        Self {
            path: PathFollowing::new(vec![
                Target::new(-1.0, -1.5, path_radius),
                Target::new( 3.0, -1.5, path_radius),
                Target::new(-2.0,  1.5, path_radius),
            ]),

            wander: Wander::new(),
        }
    }

    pub fn debug_draw(&mut self, mut view: View, actor: &Actor) {
        if false {
            let a = actor.position;
            let b = actor.position + actor.velocity;
            view.line(a, b, Color::new(1.0, 0.0, 0.0));
        }

        self.path.debug_draw(view);
        self.wander.debug_draw(view, actor);
    }
}

impl Controller for AI {
    fn run(&mut self, actor: &Actor) -> Option<Isometry2<f32>> {
        let v = self.path.target(actor)
            .map(|target| Seek::new(target).steering(actor).translation.vector)
            .map(|steering| steering + self.wander.steering(actor).translation.vector)
            .map(|steering| {
                let steering = truncate(steering, actor.max_force);
                let steering = steering / actor.mass;
                truncate(actor.velocity + steering, actor.max_speed)
            })
            .map(|velocity| velocity / actor.max_speed)
            .unwrap_or(actor.velocity);

        let rotation = UnitComplex::from_angle(v.y.atan2(v.x));
        let translation = Translation2::from_vector(v);
        Some(Isometry2::from_parts(translation, rotation))
    }
}

fn truncate(v: Vector2<f32>, max: f32) -> Vector2<f32> {
    let n = norm(&v);
    if n == 0.0 {
        zero()
    } else {
        let i = max / n;
        v * if i < 1.0 { i } else { 1.0 }
    }
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

pub trait Steering {
    fn steering(&mut self, actor: &Actor) -> Isometry2<f32>;
}

/*
    fn steering_flee(&mut self, actor: &Actor, target: Point2<f32>) -> Vector2<f32> {
        let acc = actor.max_linear_acceleration;
        let desired_velocity = (actor.position - target).normalize() * acc;
        desired_velocity - actor.velocity
    }
    */

