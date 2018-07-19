use specs::prelude::*;

#[derive(Component, Serialize, Default, Clone)]
#[storage(VecStorage)]
pub struct NetMarker(pub usize);

#[derive(Component, Serialize, Default, Clone)]
#[storage(VecStorage)]
pub struct LastProcessedInput(pub u16);

#[derive(Component, Serialize, Default, Clone)]
#[storage(VecStorage)]
pub struct Position(pub f32, pub f32);

#[derive(Component, Serialize, Default, Clone)]
#[storage(VecStorage)]
pub struct Velocity(pub f32, pub f32);

