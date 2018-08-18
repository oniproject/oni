use specs::prelude::*;
use nalgebra::{Point2, Vector2, UnitComplex};
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
    pub speed: f32, // units/s
}

impl Actor {
    pub fn spawn(position: Point2<f32>) -> Self {
        Self {
            position,
            rotation: UnitComplex::identity(),
            velocity: Vector2::zeros(),
            speed: DEFAULT_SPEED,
        }
    }

    /// Apply user's input to self entity.
    pub fn apply_input(&mut self, input: &Input) {
        self.velocity = input.stick * self.speed;
        self.position += self.velocity * input.press_time;
        self.rotation = UnitComplex::from_angle(input.rotation);
    }
}
