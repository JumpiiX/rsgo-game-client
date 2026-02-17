pub struct SpawnSystem {
    spawn_positions: Vec<(f32, f32, f32)>,
}

impl SpawnSystem {
    pub fn new() -> Self {
        Self {
            spawn_positions: vec![
                // Near central plaza - safe open area
                (20.0, 10.0, 20.0),
                
                // Near left building - front courtyard
                (-100.0, 10.0, -80.0),
                
                // Near right building - open area
                (120.0, 10.0, 80.0),
                
                // Near industrial area - loading dock
                (-180.0, 10.0, 20.0),
                
                // Near corner structure - open plaza
                (130.0, 10.0, -130.0),
            ],
        }
    }

    pub fn get_spawn_position(&self, player_count: usize) -> (f32, f32, f32) {
        let spawn_index = player_count % self.spawn_positions.len();
        self.spawn_positions[spawn_index]
    }
}