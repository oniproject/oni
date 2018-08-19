use specs::prelude::*;

#[derive(Component)]
#[storage(VecStorage)]
pub struct Node {
    pub fire: bool,
    pub fire_state: usize,
}

impl Node {
    pub fn new() -> Self {
        Self { fire: false, fire_state: 0 }
    }
}
