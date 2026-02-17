use crate::game::Player;
use std::collections::HashMap;
use std::sync::Mutex;

pub struct PlayerManager {
    players: Mutex<HashMap<String, Player>>,
}

impl PlayerManager {
    pub fn new() -> Self {
        Self {
            players: Mutex::new(HashMap::new()),
        }
    }

    pub fn add_player(&self, player: Player) {
        self.players.lock().unwrap().insert(player.id.clone(), player);
    }

    pub fn remove_player(&self, player_id: &str) {
        self.players.lock().unwrap().remove(player_id);
    }

    pub fn update_player_position(&self, player_id: &str, x: f32, y: f32, z: f32, rotation_x: f32, rotation_y: f32) {
        let mut players = self.players.lock().unwrap();
        if let Some(player) = players.get_mut(player_id) {
            player.update_position(x, y, z, rotation_x, rotation_y);
        }
    }

    pub fn get_all_players(&self) -> Vec<Player> {
        self.players.lock().unwrap().values().cloned().collect()
    }

    pub fn get_player(&self, player_id: &str) -> Option<Player> {
        self.players.lock().unwrap().get(player_id).cloned()
    }

    pub fn get_player_count(&self) -> usize {
        self.players.lock().unwrap().len()
    }
}