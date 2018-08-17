use specs::prelude::*;
use kiss3d::{
    scene::PlanarSceneNode,
    planar_camera::PlanarCamera,
    window::Window,
};
use nalgebra::{
    Point2, Vector2,
    Translation2,
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
    pub speed: f32, // units/s

    pub node: Option<Node>,
}

unsafe impl Send for Actor {}
unsafe impl Sync for Actor {}

impl Actor {
    pub fn spawn(position: Point2<f32>) -> Self {
        Self {
            position,
            rotation: UnitComplex::identity(),
            velocity: Vector2::zeros(),
            speed: DEFAULT_SPEED,
            node: None,
        }
    }

    /// Apply user's input to self entity.
    pub fn apply_input(&mut self, input: &Input) {
        self.velocity = input.stick * self.speed;
        self.position += self.velocity * input.press_time;
        self.rotation = UnitComplex::from_angle(input.rotation);
    }

    pub fn render<C>(&mut self, win: &mut Window, yy: f32, id: u32, camera: &C)
        where C: PlanarCamera
    {
        let mut pos = self.screen_pos(win, camera);
        pos.y -= yy * ACTOR_RADIUS;

        let color = if id == 0 { CURRENT } else { ANOTHER };
        let node = self.node.get_or_insert_with(|| Node::new(win, color));

        node.root.set_local_translation(Translation2::new(pos.x, pos.y));
        node.root.set_local_rotation(self.rotation);

        node.lazer.set_visible(node.fire);
        if node.fire {
            node.fire_state += 1;
            node.fire_state %= 6;
            if node.fire_state >= 3 {
                node.lazer.set_color(FIRE[0], FIRE[1], FIRE[2]);
            } else {
                node.lazer.set_color(LAZER[0], LAZER[1], LAZER[2]);
            }
        } else {
            node.fire_state = 0;
            node.lazer.set_color(LAZER[0], LAZER[1], LAZER[2]);
        }
    }

    pub fn screen_pos<C>(&self, win: &mut Window, camera: &C) -> Point2<f32>
        where C: PlanarCamera
    {
        position_to_screen(win, self.position)
    }
}

pub fn position_to_screen(win: &mut Window, position: Point2<f32>) -> Point2<f32> {
    let (w, h) = (win.width() as f32, win.height() as f32);
    let x = (position.x / 10.0) * w - w * 0.0;
    let y = (position.y / 10.0) * h + h * 0.5;
    Point2::new(x, y)
}

#[allow(dead_code)]
pub struct Node {
    root: PlanarSceneNode,
    lazer: PlanarSceneNode,
    gun: PlanarSceneNode,

    pub fire: bool,
    fire_state: usize,
}

impl Node {
    fn new(win: &mut Window, color: [f32; 3]) -> Self {
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
