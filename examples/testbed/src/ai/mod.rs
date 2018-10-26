#![allow(dead_code)]

use nalgebra::{
    Point2, Vector2,
    Point3 as Color,
    Translation2,
    Isometry2,
    UnitComplex,
    norm,
    zero,
};

use crate::{
    ui::View,
    components::{Controller, Actor},
};

//mod btree;

mod arrival;
mod seek;
mod path;
mod wander;

pub use self::seek::Seek;
pub use self::arrival::Arrival;
pub use self::path::{Target, PathFollowing};
pub use self::wander::Wander;

pub trait Boid {
    fn position(&self) -> Point2<f32>;
    fn velocity(&self) -> Vector2<f32>;
    fn rotation(&self) -> UnitComplex<f32>;

    fn mass(&self) -> f32;

    fn max_speed(&self) -> f32;
    fn max_linear_acceleration(&self) -> f32;
    fn max_force(&self) -> f32;

    fn transform(&self) -> Isometry2<f32> {
        Isometry2::from_parts(self.translation(), self.rotation())
    }

    fn translation(&self) -> Translation2<f32>  {
        Translation2::from_vector(self.position().coords)
    }
}

pub trait Steering<B: Boid> {
    fn steering(&mut self, boid: &B) -> Isometry2<f32>;
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
                Target::new( 3.0,  1.5, path_radius),
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
        self.path.target(actor)
            .map(|target| {
                let path_flow = Seek::new(target).steering(actor);
                let _wander = self.wander.steering(actor);
                /*wander */ path_flow
            })
            // apply steering
            .map(|steering| {
                let steering = steering.translation.vector;
                let steering = truncate(steering, actor.max_force());
                let steering = steering / actor.mass;
                truncate(actor.velocity() + steering, actor.max_speed)
            })
            // generate input transformation
            .map(|velocity| {
                let v = velocity / actor.max_speed();
                let rotation = UnitComplex::from_angle(v.y.atan2(v.x));
                let translation = Translation2::from_vector(v);
                Isometry2::from_parts(translation, rotation)
            })
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
