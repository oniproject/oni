use std::sync::atomic::{AtomicBool, Ordering};
use kiss3d::event::{Action, Key};

use nalgebra::{
    Point2,
    Vector2,
    Translation2,
    Isometry2,
    UnitComplex,
};

use crate::components::{Actor, Controller};

mod sender;
pub use self::sender::InputSender;

#[derive(Debug)]
pub struct Stick {
    x: InputAxis,
    y: InputAxis,
    updated: AtomicBool,

    fire: bool,
    mouse: Point2<f32>,
}

impl Default for Stick {
    fn default() -> Self {
        Self {
            x: Default::default(),
            y: Default::default(),
            updated: Default::default(),

            fire: false,

            mouse: Point2::origin(),
        }
    }
}

impl Clone for Stick {
    fn clone(&self) -> Self {
        Self {
            x: self.x,
            y: self.y,
            updated: self.updated.load(Ordering::Relaxed).into(),

            fire: self.fire,
            mouse: self.mouse,
        }
    }
}

impl Controller for Stick {
    fn run(&mut self, actor: &Actor) -> Option<Isometry2<f32>> {
        let m = (self.mouse - actor.position).normalize();
        let rotation = UnitComplex::from_cos_sin_unchecked(m.x, m.y);

        self.take_updated()
            .map(Translation2::from_vector)
            .map(|translation| Isometry2::from_parts(translation, rotation))
    }
}

impl Stick {
    pub fn velocity(&self) -> Vector2<f32> {
        let x = self.x.0.map(|v| if v { 1.0 } else { -1.0 });
        let y = self.y.0.map(|v| if v { 1.0 } else { -1.0 });

        if x.is_none() && y.is_none() {
            Vector2::zeros()
        } else {
            let (x, y) = (x.unwrap_or(0.0), y.unwrap_or(0.0));
            let vel = Vector2::new(x, y);
            vel.normalize()
        }
    }

    pub fn take_updated(&self) -> Option<Vector2<f32>> {
        let updated = if self.x.0.is_none() && self.y.0.is_none() {
            self.updated.swap(false, Ordering::Relaxed)
        } else {
            self.updated.load(Ordering::Relaxed)
        };
        if updated {
            Some(self.velocity())
        } else {
            None
        }
    }

    pub fn fire(&mut self, fire: bool) {
        self.updated.fetch_or(self.fire != fire, Ordering::Relaxed);
        self.fire = fire;
    }

    pub fn get_fire(&self) -> bool { self.fire }

    pub fn get_mouse(&self) -> Point2<f32> { self.mouse }

    pub fn mouse(&mut self, mouse: Point2<f32>) {
        self.updated.fetch_or(self.mouse != mouse, Ordering::Relaxed);
        self.mouse = mouse;
    }

    pub fn wasd(&mut self, key: Key, action: Action) {
        let last = (self.x, self.y);
        match (key, action) {
            (Key::W, action) => self.y.action(action, true ),
            (Key::S, action) => self.y.action(action, false),
            (Key::A, action) => self.x.action(action, false),
            (Key::D, action) => self.x.action(action, true ),
            (_, _) => (),
        }
        self.updated.fetch_or(last != (self.x, self.y), Ordering::Relaxed);
    }

    pub fn arrows(&mut self, key: Key, action: Action) {
        let last = (self.x, self.y);
        match (key, action) {
            (Key::Up   , action) => self.y.action(action, true ),
            (Key::Down , action) => self.y.action(action, false),
            (Key::Left , action) => self.x.action(action, false),
            (Key::Right, action) => self.x.action(action, true ),
            (_, _) => (),
        }
        self.updated.fetch_or(last != (self.x, self.y), Ordering::Relaxed);
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct InputAxis(pub Option<bool>);

impl InputAxis {
    pub fn action(&mut self, action: Action, btn: bool) {
        match (action, self.0, btn) {
            (Action::Press  , _          , _    ) => self.0 = Some(btn),

            (Action::Release, None       , _    ) |
            (Action::Release, Some(true ), true ) |
            (Action::Release, Some(false), false) => self.0 = None,

            (Action::Release, Some(true ), false) |
            (Action::Release, Some(false), true ) => (),
        }
    }
}
