use nalgebra::{Point2, Vector2};

#[derive(Serialize, Deserialize, Clone, Copy, Debug)]
pub struct Input {
    pub stick: Stick,
    pub press_time: f32,
    pub sequence: usize,
    pub entity_id: usize,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, Default)]
pub struct Stick {
    pub x: InputAxis,
    pub y: InputAxis,
}

impl Stick {
    pub fn any(&self) -> bool {
        self.x.0.is_some() || self.y.0.is_some()
    }
    pub fn velocity(&self, speed: f32) -> Option<Vector2<f32>> {
        let x = self.x.0.map(|v| if v { 1.0 } else { -1.0 });
        let y = self.y.0.map(|v| if v { 1.0 } else { -1.0 });
        if x.is_none() && y.is_none() {
            None
        } else {
            let (x, y) = (x.unwrap_or(0.0), y.unwrap_or(0.0));
            let vel = Vector2::new(x, y);
            Some(vel.normalize() * speed)
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, Default)]
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

pub struct InputState {
    pub stick: Stick,

    // Data needed for reconciliation.
    pub sequence: usize,
    pub pending_inputs: Vec<Input>,
}

impl InputState {
    pub fn new() -> Self {
        Self {
            stick: Stick::default(),

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
