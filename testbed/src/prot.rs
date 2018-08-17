use nalgebra::{Point2, Vector2, UnitComplex};

#[derive(Clone, Debug)]
pub struct Input {
    pub stick: Vector2<f32>,
    pub rotation: f32,
    pub press_time: f32,
    pub sequence: usize,
    pub entity_id: usize,
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
