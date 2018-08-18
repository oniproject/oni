use kiss3d::{
    window::Window,
    scene::PlanarSceneNode,
};
use nalgebra::Translation2;
use specs::prelude::*;
use crate::consts::*;

#[derive(Component)]
#[storage(VecStorage)]
pub struct Node {
    pub root: PlanarSceneNode,
    pub lazer: PlanarSceneNode,
    pub gun: PlanarSceneNode,

    pub fire: bool,
    pub fire_state: usize,
}

impl Node {
    pub fn new(win: &mut Window, color: [f32; 3]) -> Self {
        let mut root = win.add_rectangle(ACTOR_RADIUS * 1.5, ACTOR_RADIUS * 1.5);
        root.set_color(color[0], color[1], color[2]);

        let mut lazer = root.add_rectangle(7.0, 0.05);
        lazer.set_color(LAZER[0], LAZER[1], LAZER[2]);
        lazer.set_local_translation(Translation2::new(85.0, 0.0));

        let mut gun = root.add_rectangle(1.0, 0.30);
        gun.set_color(GUN[0], GUN[1], GUN[2]);
        gun.set_local_translation(Translation2::new(10.0, 0.0));

        Self { root, lazer, gun, fire: false, fire_state: 0 }
    }
}

unsafe impl Send for Node {}
unsafe impl Sync for Node {}
