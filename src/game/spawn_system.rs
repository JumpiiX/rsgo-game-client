#[derive(Debug)]
pub struct SpawnSystem {
    attacker_spawn_positions: Vec<(f32, f32, f32)>,
    defender_spawn_positions: Vec<(f32, f32, f32)>,
}

impl SpawnSystem {
    pub fn new() -> Self {
        Self {
            // Attacker spawn positions (bottom spawn area at z = -300)
            attacker_spawn_positions: vec![
                (0.0, 10.0, -300.0),
                (-15.0, 10.0, -300.0),
                (15.0, 10.0, -300.0),
                (-30.0, 10.0, -300.0),
                (30.0, 10.0, -300.0),
            ],
            // Defender spawn positions (top spawn area at z = 300)
            defender_spawn_positions: vec![
                (0.0, 10.0, 300.0),
                (-15.0, 10.0, 300.0),
                (15.0, 10.0, 300.0),
                (-30.0, 10.0, 300.0),
                (30.0, 10.0, 300.0),
            ],
        }
    }

    pub fn get_spawn_position(&self, player_count: usize) -> (f32, f32, f32) {
        // For now, alternate between attacker and defender spawns
        // In the future, this should be based on team assignment
        if player_count % 2 == 0 {
            let spawn_index = (player_count / 2) % self.attacker_spawn_positions.len();
            self.attacker_spawn_positions[spawn_index]
        } else {
            let spawn_index = (player_count / 2) % self.defender_spawn_positions.len();
            self.defender_spawn_positions[spawn_index]
        }
    }
    
    pub fn get_team_spawn_position(&self, team: &str, team_player_count: usize) -> (f32, f32, f32) {
        match team {
            "orange" | "attacker" => {
                let spawn_index = team_player_count % self.attacker_spawn_positions.len();
                self.attacker_spawn_positions[spawn_index]
            },
            "red" | "defender" => {
                let spawn_index = team_player_count % self.defender_spawn_positions.len();
                self.defender_spawn_positions[spawn_index]
            },
            _ => {
                // Default to attacker spawn if team is unknown
                let spawn_index = team_player_count % self.attacker_spawn_positions.len();
                self.attacker_spawn_positions[spawn_index]
            }
        }
    }
}