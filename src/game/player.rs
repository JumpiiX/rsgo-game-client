use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Player {
    pub id: String,
    pub name: String,
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub rotation_x: f32,
    pub rotation_y: f32,
    pub health: i32,
    pub alive: bool,
}

impl Player {
    pub fn new(id: String, name: String, spawn_pos: (f32, f32, f32)) -> Self {
        Self {
            id,
            name,
            x: spawn_pos.0,
            y: spawn_pos.1,
            z: spawn_pos.2,
            rotation_x: 0.0,
            rotation_y: 0.0,
            health: 100,
            alive: true,
        }
    }

    pub fn update_position(&mut self, x: f32, y: f32, z: f32, rotation_x: f32, rotation_y: f32) {
        self.x = x;
        self.y = y;
        self.z = z;
        self.rotation_x = rotation_x;
        self.rotation_y = rotation_y;
    }
}