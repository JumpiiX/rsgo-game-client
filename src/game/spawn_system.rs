pub struct SpawnSystem {
    spawn_positions: Vec<(f32, f32, f32)>,
}

impl SpawnSystem {
    pub fn new() -> Self {
        Self {
            spawn_positions: vec![
                (20.0, 10.0, 20.0),
                (-100.0, 10.0, -80.0),
                (120.0, 10.0, 80.0),
                (-180.0, 10.0, 20.0),
                (130.0, 10.0, -130.0),
            ],
        }
    }

    pub fn get_spawn_position(&self, player_count: usize) -> (f32, f32, f32) {
        let spawn_index = player_count % self.spawn_positions.len();
        self.spawn_positions[spawn_index]
    }
}