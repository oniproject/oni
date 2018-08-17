use std::sync::atomic::{AtomicBool, Ordering};
use nalgebra::{Point2, Vector2, UnitComplex};
use kiss3d::event::{Action, Key};

#[derive(Clone, Debug)]
pub struct Input {
    pub stick: Vector2<f32>,
    pub rotation: f32,
    pub press_time: f32,
    pub sequence: usize,
    pub entity_id: usize,
}

#[derive(Debug, Default)]
pub struct Stick {
    pub x: InputAxis,
    pub y: InputAxis,
    pub rotation: f32,
    updated: AtomicBool,
}

impl Clone for Stick {
    fn clone(&self) -> Self {
        Self {
            x: self.x,
            y: self.y,
            rotation: 0.0,
            updated: self.updated.load(Ordering::Relaxed).into(),
        }
    }
}

impl Stick {
    pub fn from_velocity(velocity: Vector2<f32>) -> Self {
        let x = if velocity.x == 0.0 {
            InputAxis(None)
        } else {
            InputAxis(Some(velocity.x > 0.0))
        };

        let y = if velocity.y == 0.0 {
            InputAxis(None)
        } else {
            InputAxis(Some(velocity.y > 0.0))
        };

        Self { x, y, rotation: 0.0, updated: AtomicBool::new(true) }
    }

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

    pub fn rotate(&mut self, rotation: f32) {
        self.updated.fetch_or(self.rotation != rotation, Ordering::Relaxed);
        self.rotation = rotation;
    }

    pub fn wasd(&mut self, key: Key, action: Action) {
        let last = (self.x, self.y);
        match (key, action) {
            (Key::W, action) => self.y.action(action == Action::Press, false),
            (Key::S, action) => self.y.action(action == Action::Press, true ),
            (Key::A, action) => self.x.action(action == Action::Press, false),
            (Key::D, action) => self.x.action(action == Action::Press, true ),
            (_, _) => (),
        }
        self.updated.fetch_or(last != (self.x, self.y), Ordering::Relaxed);
    }

    pub fn arrows(&mut self, key: Key, action: Action) {
        let last = (self.x, self.y);
        match (key, action) {
            (Key::Up   , action) => self.y.action(action == Action::Press, false),
            (Key::Down , action) => self.y.action(action == Action::Press, true ),
            (Key::Left , action) => self.x.action(action == Action::Press, false),
            (Key::Right, action) => self.x.action(action == Action::Press, true ),
            (_, _) => (),
        }
        self.updated.fetch_or(last != (self.x, self.y), Ordering::Relaxed);
    }
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct InputAxis(pub Option<bool>);

impl InputAxis {
    pub fn action(&mut self, action: bool, btn: bool) {
        match (action, self.0, btn) {
            (true , _          , _    ) => self.0 = Some(btn),

            (false, None       , _    ) |
            (false, Some(true ), true ) |
            (false, Some(false), false) => self.0 = None,

            (false, Some(true ), false) |
            (false, Some(false), true ) => (),
        }
    }
}

// Data needed for reconciliation.
pub struct Reconciliation {
    pub sequence: usize,
    pub pending_inputs: Vec<Input>,
}

impl Reconciliation {
    pub fn new() -> Self {
        Self {
            sequence: 1,
            pending_inputs: Vec::new(),
        }
    }

    pub fn non_acknowledged(&self) -> usize {
        self.pending_inputs.len()
    }

    pub fn save(&mut self, input: Input) {
        self.pending_inputs.push(input);
    }

    pub fn reconciliation(
        &mut self,
        entity: &mut crate::actor::Actor,
        position: Point2<f32>,
        input_ack: usize,
    ) {
        // Received the authoritative position
        // of self client's entity.
        entity.position = position;

        if false {
            // Reconciliation is disabled,
            // so drop all the saved inputs.
            self.pending_inputs.clear();
            return;
        }

        // Server Reconciliation.
        // Re-apply all the inputs not yet processed by the server.

        // Already processed.
        // Its effect is already taken into
        // account into the world update
        // we just got, so we can drop it.
        self.pending_inputs.retain(|i| i.sequence > input_ack);

        // Not processed by the server yet.
        // Re-apply it.
        for input in &self.pending_inputs {
            entity.apply_input(input);
        }
    }
}
