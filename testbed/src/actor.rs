use std::{
    time::Instant,
    collections::VecDeque,
};
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
    wrap,
};
use crate::{
    input::*,
    consts::*,
    util::*,
};

struct State {
    time: Instant,
    position: Point2<f32>,
    rotation: UnitComplex<f32>,
    velocity: Vector2<f32>,
}

impl State {
    fn interpolate_linear(a: &Self, b: &Self, time: Instant) -> Point2<f32> {
        a.position + (b.position - a.position) *
            duration_to_secs(  time - a.time) /
            duration_to_secs(b.time - a.time)
    }

    fn interpolate_angular(a: &Self, b: &Self, time: Instant) -> UnitComplex<f32> {
        use std::f32::consts::PI;
        let (from, to) = (a.rotation.angle(), b.rotation.angle());
        let angle = from + wrap(to - from, -PI, PI) *
            duration_to_secs(  time - a.time) /
            duration_to_secs(b.time - a.time);
        UnitComplex::from_angle(angle)
    }
}

#[derive(Component)]
#[storage(VecStorage)]
pub struct Actor {
    pub position: Point2<f32>,
    pub rotation: UnitComplex<f32>,
    pub velocity: Vector2<f32>,
    pub speed: f32, // units/s

    pub node: Option<Node>,

    buf: VecDeque<State>,
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
            buf: VecDeque::new(),
            node: None,
        }
    }

    /// Apply user's input to self entity.
    pub fn apply_input(&mut self, input: &Input) {
        self.velocity = input.stick * self.speed;
        self.position += self.velocity * input.press_time;
        self.rotation = UnitComplex::from_angle(input.rotation);
    }

    /// Drop older positions.
    pub fn drop_older(&mut self, than: Instant) {
        while self.buf.len() >= 2 && self.buf[1].time <= than {
            self.buf.pop_front();
        }
    }

    pub fn push_state(&mut self, time: Instant, state: &EntityState) {
        self.buf.push_back(State {
            time,
            position: state.position,
            velocity: state.velocity,
            rotation: state.rotation,
        });
    }

    pub fn interpolate(&mut self, time: Instant) {
        self.drop_older(time);

        // Find the two authoritative positions surrounding the rendering time.
        // Interpolate between the two surrounding authoritative positions.
        if self.buf.len() >= 2 {
            let (a, b) = (&self.buf[0], &self.buf[1]);
            if a.time <= time && time <= b.time {
                self.position = State::interpolate_linear(a, b, time);
                self.rotation = State::interpolate_angular(a, b, time);
            }
        }
    }

    pub fn render<C>(&mut self, win: &mut Window, yy: f32, id: u32, camera: &C)
        where C: PlanarCamera
    {
        if self.node.is_none() {
            let mut root = win.add_rectangle(ACTOR_RADIUS * 1.5, ACTOR_RADIUS * 1.5);

            if id == 0 {
                root.set_color(CURRENT[0], CURRENT[1], CURRENT[2]);
            } else {
                root.set_color(ANOTHER[0], ANOTHER[1], ANOTHER[2]);
            }

            let mut lazer = root.add_rectangle(7.0, 0.05);
            lazer.set_color(LAZER[0], LAZER[1], LAZER[2]);
            lazer.set_local_translation(Translation2::new(85.0, 0.0));

            let mut gun = root.add_rectangle(1.0, 0.30);
            gun.set_color(GUN[0], GUN[1], GUN[2]);
            gun.set_local_translation(Translation2::new(10.0, 0.0));

            self.node = Some(Node { root, lazer, gun, fire: false, fire_state: 0 });
        }

        let node = self.node.as_mut().unwrap();
        let mut pos = position_to_screen(win, self.position);
        pos.y -= yy * ACTOR_RADIUS;

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
}

pub fn position_to_screen(win: &mut Window, position: Point2<f32>) -> Point2<f32> {
    let (w, h) = (win.width() as f32, win.height() as f32);
    let x =  (position.x / 10.0) * w - w * 0.5;
    let y = -(position.y / 10.0) * h + h * 0.5;
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

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct WorldState {
    pub last_processed_input: usize,
    pub states: Vec<EntityState>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct EntityState {
    pub entity_id: u16,
    pub position: Point2<f32>,
    pub velocity: Vector2<f32>,
    pub rotation: UnitComplex<f32>,
}
