use std::{
    time::Instant,
    collections::VecDeque,
};
use kiss3d::{
    scene::PlanarSceneNode,
    window::Window,
};
use nalgebra::{
    Id,
    Point2, Vector2,
    Translation2,
    UnitComplex,
};
use crate::util::duration_to_secs;
use crate::consts::*;

struct State {
    time: Instant,
    position: Point2<f32>,
}

impl State {
    fn interpolate(&self, other: &Self, time: Instant) -> Point2<f32> {
        self.position + (other.position - self.position) *
            duration_to_secs(      time - self.time) /
            duration_to_secs(other.time - self.time)
    }
}

pub struct Actor {
    id: usize,
    position: Point2<f32>,
    speed: Vector2<f32>, // units/s
    buf: VecDeque<State>,
    pub node: Option<Node>,
}

impl Actor {
    pub fn new(id: usize, x: f32) -> Self {
        Self {
            position: Point2::new(x, 0.0),
            speed: Vector2::new(DEFAULT_SPEED, DEFAULT_SPEED),
            id,
            buf: VecDeque::new(),
            node: None,
        }
    }

    pub fn id(&self) -> usize { self.id }

    pub fn position(&self) -> Point2<f32> { self.position }
    pub fn set_position(&mut self, position: Point2<f32>) {
        self.position = position;
    }

    /// Apply user's input to self entity.
    pub fn apply_input(&mut self, press_time: f32) {
        self.position += self.speed * press_time;
    }

    /// Drop older positions.
    pub fn drop_older(&mut self, than: Instant) {
        while self.buf.len() >= 2 && self.buf[1].time <= than {
            self.buf.pop_front();
        }
    }

    pub fn push_position(&mut self, time: Instant, position: Point2<f32>) {
        self.buf.push_back(State { time, position });
    }

    pub fn interpolate(&mut self, time: Instant) {
        self.drop_older(time);

        // Find the two authoritative positions surrounding the rendering time.
        // Interpolate between the two surrounding authoritative positions.
        if self.buf.len() >= 2 {
            let (a, b) = (&self.buf[0], &self.buf[1]);
            if a.time <= time && time <= b.time {
                self.position = a.interpolate(b, time);
            }
        }
    }

    pub fn render(&mut self, win: &mut Window, yy: f32, mouse: Point2<f32>) {
        if self.node.is_none() {
            let mut root = win.add_rectangle(ACTOR_RADIUS * 1.5, ACTOR_RADIUS * 1.5);

            if self.id == 0 {
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
        let (w, h) = (win.width() as f32, win.height() as f32);
        let x = (self.position.x / 10.0) * w - w * 0.5;
        let y = -(yy * ACTOR_RADIUS) + h * 0.5 - ACTOR_RADIUS;
        //let y: f32 = (              y / 10.0) * h; // - h * 1.0;

        node.root.set_local_translation(Translation2::new(x, y));

        if self.id == 0 {
            let m = (mouse - Point2::new(x, y)).normalize();
            let rot = UnitComplex::from_cos_sin_unchecked(m.x, m.y);

            node.root.set_local_rotation(rot);
        }

        node.lazer.set_visible(node.fire);
        if node.fire {
            node.fire_state += 1;
            node.fire_state %= 3;
            if node.fire_state != 0 {
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

pub struct Node {
    root: PlanarSceneNode,
    lazer: PlanarSceneNode,
    gun: PlanarSceneNode,

    pub fire: bool,
    fire_state: usize,
}