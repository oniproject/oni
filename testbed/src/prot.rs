use nalgebra::Point2;

#[derive(Serialize, Deserialize, Clone, Copy, Debug)]
pub struct Input {
    pub press_time: f32,
    pub sequence: usize,
    pub entity_id: usize,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug)]
pub struct WorldState {
    pub entity_id: usize,
    pub position: Point2<f32>,
    pub last_processed_input: usize,
}
