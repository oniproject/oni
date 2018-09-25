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
    actors: WriteStorage<'a, Actor>,
    states: WriteStorage<'a, StateBuffer>,
    node: ReadExpect<'a, NetNode>,
}

impl<'a> System<'a> for Interpolation {
    type SystemData = InterpolationData<'a>;

    fn run(&mut self, mut data: Self::SystemData) {
        oni::trace::scope![Interpolation];

        decelerator!();

        // Compute render time.
        let render_time = Instant::now() - RENDER_TIME;

        let me: Option<Entity> = data.node.me;
        let actors = (&*data.entities, &mut data.actors, &mut data.states).join()
            // No point in interpolating self client's entity.
            .filter(|(e, _, _)| Some(*e) != me);

        for (e, actor, state) in actors {
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
