use std::time::Instant;
use specs::prelude::*;
use crate::{
    components::*,
    consts::*,
};

use super::StateBuffer;

pub struct Interpolation;

#[derive(SystemData)]
pub struct InterpolationData<'a> {
    entities: Entities<'a>,
    me: ReadExpect<'a, Entity>,
    actors: WriteStorage<'a, Actor>,
    states: WriteStorage<'a, StateBuffer>,
}

impl<'a> System<'a> for Interpolation {
    type SystemData = InterpolationData<'a>;

    fn run(&mut self, mut data: Self::SystemData) {
        oni::trace::scope![Interpolation];

        // Compute render time.
        let render_time = Instant::now() - RENDER_TIME;

        let me = *data.me;
        let actors = (&*data.entities, &mut data.actors, &mut data.states).join()
            // No point in interpolating self client's entity.
            //.filter_map(|(e, a, s)| if e == me { None } else { Some((a, s)) });
            .filter(|(e, _, _)| *e != me);

        for (e, actor, state) in actors {
            state.drop_older(render_time);
            if !state.interpolate(render_time, actor) {
                //unimplemented!("extrapolation")
                println!("unimplemented extrapolation: me: {}, e: {}",
                         me.id(), e.id());
            }
        }
    }
}
