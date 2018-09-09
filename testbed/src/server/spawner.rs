use specs::{
    prelude::*,
    saveload::Marker,
};
use nalgebra::Point2;
use crate::components::*;

pub struct Spawner {
    pub spawn_index: usize,
    pub spawn_points: Vec<Point2<f32>>,
}

impl Spawner {
    pub fn new() -> Self {
        Self {
            spawn_index: 0,
            spawn_points: vec![
                Point2::new(-3.0, 0.0),
                Point2::new( 3.0, 0.0),
            ],
        }
    }
}

#[derive(SystemData)]
pub struct Data<'a> {
    entities: Entities<'a>,
    mark: ReadStorage<'a, NetMarker>,
    actors: ReadStorage<'a, Actor>,
    lazy: ReadExpect<'a, LazyUpdate>,
}

impl<'a> System<'a> for Spawner {
    type SystemData = Data<'a>;

    fn run(&mut self, data: Self::SystemData) {
        oni::trace::scope![spawner];

        decelerator!();

        for (e, _, _) in (&*data.entities, !(&data.actors), &data.mark).join() {
            let pos = self.spawn_points[self.spawn_index];
            data.lazy.insert(e, Actor::spawn(pos));
            self.spawn_index += 1;
            self.spawn_index %= self.spawn_points.len();
        }
    }
}
