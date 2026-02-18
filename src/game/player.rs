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
    pub kills: i32,
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
            kills: 0,
        }
    }

    pub fn update_position(&mut self, x: f32, y: f32, z: f32, rotation_x: f32, rotation_y: f32) {
        self.x = x;
        self.y = y;
        self.z = z;
        self.rotation_x = rotation_x;
        self.rotation_y = rotation_y;
    }
    
    pub fn take_damage(&mut self, damage: i32) -> bool {
        self.health -= damage;
        if self.health <= 0 {
            self.health = 0;
            self.alive = false;
            true  // Player died
        } else {
            false  // Player still alive
        }
    }
    
    pub fn add_kill(&mut self) {
        self.kills += 1;
    }
    
    pub fn respawn(&mut self, spawn_pos: (f32, f32, f32)) {
        self.x = spawn_pos.0;
        self.y = spawn_pos.1;
        self.z = spawn_pos.2;
        self.health = 100;
        self.alive = true;
    }
}