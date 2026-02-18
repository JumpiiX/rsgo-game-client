use serde::{Deserialize, Serialize};
use crate::game::Player;

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ClientMessage {
    #[serde(rename = "join")]
    Join { name: String },
    #[serde(rename = "move")]
    Move { x: f32, y: f32, z: f32, rotation_x: f32, rotation_y: f32 },
    #[serde(rename = "shoot")]
    Shoot { start_x: f32, start_y: f32, start_z: f32, target_x: f32, target_y: f32, target_z: f32 },
    #[serde(rename = "hit")]
    Hit { target_player_id: String, killed: bool },
    #[serde(rename = "respawn")]
    Respawn,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ServerMessage {
    #[serde(rename = "welcome")]
    Welcome { player_id: String },
    #[serde(rename = "player_joined")]
    PlayerJoined { player: Player },
    #[serde(rename = "player_left")]
    PlayerLeft { player_id: String },
    #[serde(rename = "player_moved")]
    PlayerMoved { player_id: String, x: f32, y: f32, z: f32, rotation_x: f32, rotation_y: f32 },
    #[serde(rename = "player_shot")]
    PlayerShot { shooter_id: String, start_x: f32, start_y: f32, start_z: f32, target_x: f32, target_y: f32, target_z: f32 },
    #[serde(rename = "player_hit")]
    PlayerHit { player_id: String, damage: i32, health: i32 },
    #[serde(rename = "player_died")]
    PlayerDied { player_id: String, killer_id: String },
    #[serde(rename = "player_respawned")]
    PlayerRespawned { player: Player },
}