use specs::prelude::*;
use nalgebra::{
    Point2,
    Vector2,
    Translation2,
    Isometry2,
    UnitComplex,
};
use crate::{
    prot::*,
    consts::*,
};

#[derive(Component)]
#[storage(VecStorage)]
pub struct Actor {
    pub position: Point2<f32>,
    pub rotation: UnitComplex<f32>,
    pub velocity: Vector2<f32>,
    pub max_speed: f32, // units/s

    pub max_linear_acceleration: f32,
    pub max_force: f32,
    pub mass: f32,

    pub fire: bool,
    pub damage: bool,
}

impl crate::ai::Boid for Actor {
    fn position(&self) -> Point2<f32> { self.position }
    fn rotation(&self) -> UnitComplex<f32> { self.rotation }
    fn velocity(&self) -> Vector2<f32> { self.velocity }
    fn max_speed(&self) -> f32 { self.max_speed }

    fn max_linear_acceleration(&self) -> f32 { self.max_linear_acceleration }
    fn max_force(&self) -> f32 { self.max_force }
    fn mass(&self) -> f32 { self.mass }
}


pub trait Controller {
    fn run(&mut self, actor: &Actor) -> Option<Isometry2<f32>>;
}

impl Actor {
    pub fn spawn(position: Point2<f32>) -> Self {
        Self {
            position,
            rotation: UnitComplex::identity(),
            velocity: Vector2::zeros(),
            max_speed: DEFAULT_SPEED,

            max_linear_acceleration: 4.0,
            max_force: 2.8,
            mass: 10.0,

            fire: false,
            damage: false,
        }
    }

    pub fn transform(&self) -> Isometry2<f32> {
        Isometry2::from_parts(self.translation(), self.rotation)
    }

    pub fn translation(&self) -> Translation2<f32>  {
        Translation2::from_vector(self.position.coords)
    }

    /// Apply user's input to self entity.
    pub fn apply_input(&mut self, input: &Input) {
        let stick: Vector2<f32> = input.stick.into();
        self.velocity = stick * self.max_speed;
        self.position += self.velocity * input.press_delta;
        self.rotation = UnitComplex::from_angle(input.rotation);

        self.fire = input.fire;
    }
}
