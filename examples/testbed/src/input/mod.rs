use std::sync::atomic::{AtomicBool, Ordering};
use kiss2d::{Canvas, Key};

use nalgebra::{
    Point2,
    Vector2,
    Translation2,
    Isometry2,
    UnitComplex,
};

use crate::components::{Actor, Controller};

mod sender;
mod receiver;
//pub use self::sender::InputSender;

pub type InputSender = self::sender::Sender<crate::prot::InputSample>;

pub enum Action {
    Press,
    Release,
}

#[derive(Clone, Copy, PartialEq, Eq, Default, Debug)]
struct WASD {
    w: bool,
    a: bool,
    s: bool,
    d: bool,
}

macro handle($last:expr, $canvas:expr, $key:expr) {{
    let current = $canvas.is_keydown($key);
    let last = std::mem::replace($last, current);
    match (last, current) {
        (false, true) => Some(Action::Press),
        (true, false) => Some(Action::Release),
        _ => None,
    }
}}

impl WASD {
    fn new() -> Self {
        Self {
            w: false,
            a: false,
            s: false,
            d: false,
        }
    }

    fn w(&mut self, canvas: &mut Canvas) -> Option<Action> { handle!(&mut self.w, canvas, Key::W) }
    fn a(&mut self, canvas: &mut Canvas) -> Option<Action> { handle!(&mut self.a, canvas, Key::A) }
    fn s(&mut self, canvas: &mut Canvas) -> Option<Action> { handle!(&mut self.s, canvas, Key::S) }
    fn d(&mut self, canvas: &mut Canvas) -> Option<Action> { handle!(&mut self.d, canvas, Key::D) }
}

#[derive(Debug)]
pub struct Stick {
    updated: AtomicBool,
    x: InputAxis,
    y: InputAxis,
    fire: bool,
    mouse: Point2<f32>,
    wasd: WASD,
}

impl Default for Stick {
    fn default() -> Self {
        Self {
            x: Default::default(),
            y: Default::default(),
            updated: Default::default(),
            wasd: Default::default(),

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
            wasd: self.wasd.clone(),

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

    pub fn wasd(&mut self, canvas: &mut Canvas) {
        let last = (self.x, self.y);

        if let Some(action) = self.wasd.w(canvas) { self.y.action(action, false) }
        if let Some(action) = self.wasd.a(canvas) { self.x.action(action, false) }
        if let Some(action) = self.wasd.s(canvas) { self.y.action(action, true ) }
        if let Some(action) = self.wasd.d(canvas) { self.x.action(action, true ) }

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
