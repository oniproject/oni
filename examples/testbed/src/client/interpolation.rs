use std::time::Instant;
use specs::prelude::*;
use crate::{
    components::*,
    consts::*,
};

use super::StateBuffer;

pub struct Interpolation;

impl<'a> System<'a> for Interpolation {
    type SystemData = (
        WriteStorage<'a, Actor>,
        WriteStorage<'a, StateBuffer>,
        ReadStorage<'a, InterpolationMarker>,
    );

    fn run(&mut self, (mut actors, mut states, marks): Self::SystemData) {
        oni_trace::scope![Interpolation];

        decelerator!();

        // Compute render time.
        let render_time = Instant::now() - RENDER_TIME;

        for (actor, state, _) in (&mut actors, &mut states, &marks).join() {
            state.drop_older(render_time);
            if !state.interpolate(render_time, actor) {
                //actor.position = state.position.into();
                //unimplemented!("extrapolation")
                /*
                println!("unimplemented extrapolation: me: {:?}, e: {}",
                         me, e.id());
                */
            }
        }
    }
}
