use std::f32::consts::{PI, FRAC_PI_2};

use nalgebra::{
    UnitComplex,
    Point2,
    wrap,
};

use specs::prelude::*;
use std::time::Instant;
use oni::simulator::Socket;
use crate::{
    components::*,
    prot::*,
    prot::Endpoint,
    consts::*,
};

/*
struct Bot {
    position: Point2<f32>,
    rotation: f32,

    direction: f32,
    turn_speed: f32,
    speed: f32,
}
*/


#[derive(Component)]
#[storage(DenseVecStorage)]
pub struct StupidBot {
    position: Point2<f32>,
    direction: f32,
    turn_speed: f32,
}

impl StupidBot {
    pub fn new() -> Self {
        let x: f32 = rand::random();
        let y: f32 = rand::random();
        let r: f32 = rand::random();
        let s: f32 = rand::random();
        Self {
            position: Point2::new(x - 0.5, y - 0.5),
            direction: r * PI * 2.0,
            turn_speed: s - 0.8,
        }
    }
}

pub struct Stupid;

#[derive(SystemData)]
pub struct StupidData<'a> {
    actors: WriteStorage<'a, Actor>,
    stupid: WriteStorage<'a, StupidBot>,
}

impl<'a> System<'a> for Stupid {
    type SystemData = StupidData<'a>;

    fn run(&mut self, mut data: Self::SystemData) {
        let speed = 1.0 / 10.0;
        let w2 = 6.0;
        let h2 = 2.0;
        for (actor, bot) in (&mut data.actors, &mut data.stupid).join() {

            bot.direction += bot.turn_speed * 0.1;

            let (s, c) = bot.direction.sin_cos();

            bot.position.x += s * speed;
            bot.position.y += c * speed;

            // wrap the bots around as the crawl
            bot.position.x = wrap(bot.position.x, -w2, w2);
            bot.position.y = wrap(bot.position.y, -h2, h2);

            let angle = -bot.direction + FRAC_PI_2;
            actor.rotation = UnitComplex::from_angle(angle);
            //actor.rotation = bot.direction;
            actor.position = bot.position;
        }
    }
}
