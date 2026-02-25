use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};

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
    pub shield: i32,
    pub alive: bool,
    pub kills: i32,
    #[serde(skip)]
    pub last_hit_time: Option<Instant>,
    #[serde(skip)]
    pub shield_regen_active: bool,
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
            shield: 100,
            alive: true,
            kills: 0,
            last_hit_time: None,
            shield_regen_active: false,
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
        // Update last hit time for shield regeneration
        self.last_hit_time = Some(Instant::now());
        self.shield_regen_active = false;
        
        let mut remaining_damage = damage;
        
        // Apply damage to shield first
        if self.shield > 0 {
            let shield_damage = remaining_damage.min(self.shield);
            self.shield -= shield_damage;
            remaining_damage -= shield_damage;
        }
        
        // Apply remaining damage to health
        if remaining_damage > 0 {
            self.health -= remaining_damage;
        }
        
        // Check if player died
        if self.health <= 0 {
            self.health = 0;
            self.alive = false;
            true  // Player died
        } else {
            false  // Player still alive
        }
    }
    
    pub fn update_shield_regen(&mut self) -> bool {
        let mut shield_changed = false;
        
        if let Some(last_hit) = self.last_hit_time {
            let time_since_hit = last_hit.elapsed();
            
            // Start shield regen after 5 seconds
            if time_since_hit >= Duration::from_secs(5) && self.shield < 50 && self.alive {
                if !self.shield_regen_active {
                    self.shield_regen_active = true;
                }
                
                let old_shield = self.shield;
                // Regenerate 1 shield per 100ms (10 shield per second), cap at 50
                self.shield = (self.shield + 1).min(50);
                
                if self.shield != old_shield {
                    shield_changed = true;
                }
                
                // Stop at max shield
                if self.shield >= 50 {
                    self.shield_regen_active = false;
                }
            }
        } else if self.shield < 50 && self.alive {
            // If never hit, regenerate shield to 50 (spawn protection)
            let old_shield = self.shield;
            self.shield = 50;
            shield_changed = old_shield != self.shield;
        }
        
        shield_changed
    }
    
    pub fn add_kill(&mut self) {
        self.kills += 1;
    }
    
    pub fn respawn(&mut self, spawn_pos: (f32, f32, f32)) {
        self.x = spawn_pos.0;
        self.y = spawn_pos.1;
        self.z = spawn_pos.2;
        self.health = 100;
        self.shield = 100;
        self.alive = true;
        self.last_hit_time = None;
        self.shield_regen_active = false;
    }
}