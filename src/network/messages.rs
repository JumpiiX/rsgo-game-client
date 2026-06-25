use serde::{Deserialize, Serialize};
use crate::game::Player;

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ClientMessage {
    #[serde(rename = "join")]
    Join { name: String },
    #[serde(rename = "create_team_lobby")]
    CreateTeamLobby { name: String },
    #[serde(rename = "join_team")]
    JoinTeam { team: String },
    #[serde(rename = "start_team_game")]
    StartTeamGame,
    #[serde(rename = "move")]
    Move { x: f32, y: f32, z: f32, rotation_x: f32, rotation_y: f32 },
    #[serde(rename = "shoot")]
    Shoot { start_x: f32, start_y: f32, start_z: f32, target_x: f32, target_y: f32, target_z: f32 },
    #[serde(rename = "hit")]
    Hit { target_player_id: String, killed: bool },
    #[serde(rename = "respawn")]
    Respawn,
    #[serde(rename = "plant_bomb")]
    PlantBomb { 
        #[serde(default)] position_x: Option<f32>,
        #[serde(default)] position_z: Option<f32>
    },
    #[serde(rename = "defuse_bomb")]
    DefuseBomb,
    #[serde(rename = "drop_bomb")]
    DropBomb,
    #[serde(rename = "pickup_bomb")]
    PickupBomb,
    #[serde(rename = "place_structure")]
    PlaceStructure { structure_type: String, x: f32, y: f32, z: f32 },
    // A bullet punched a hole in a destructible wall. wall_x/wall_z identify the
    // wall (its center); local_x is along the wall's width, world_y is the hit's
    // absolute height. The server records it + broadcasts so all clients agree.
    #[serde(rename = "wall_hit")]
    WallHit { wall_x: f32, wall_z: f32, local_x: f32, world_y: f32, radius: f32 },
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ServerMessage {
    #[serde(rename = "welcome")]
    Welcome { player_id: String, lobby_id: String, game_mode: String },
    #[serde(rename = "team_lobby_created")]
    TeamLobbyCreated { lobby_id: String },
    #[serde(rename = "team_update")]
    TeamUpdate { 
        orange_team: Vec<TeamPlayer>, 
        red_team: Vec<TeamPlayer>,
        can_start: bool 
    },
    #[serde(rename = "game_started")]
    GameStarted { game_mode: String },
    #[serde(rename = "player_joined")]
    PlayerJoined { player: Player },
    #[serde(rename = "player_left")]
    PlayerLeft { player_id: String },
    #[serde(rename = "player_moved")]
    PlayerMoved { player_id: String, x: f32, y: f32, z: f32, rotation_x: f32, rotation_y: f32 },
    #[serde(rename = "player_shot")]
    PlayerShot { shooter_id: String, start_x: f32, start_y: f32, start_z: f32, target_x: f32, target_y: f32, target_z: f32 },
    #[serde(rename = "player_hit")]
    PlayerHit { player_id: String, damage: i32, health: i32, shield: i32 },
    #[serde(rename = "player_died")]
    PlayerDied { player_id: String, killer_id: String, victim_name: String, killer_name: String },
    #[serde(rename = "player_respawned")]
    PlayerRespawned { player: Player },
    #[serde(rename = "shield_update")]
    ShieldUpdate { player_id: String, shield: i32 },
    #[serde(rename = "money_update")]
    MoneyUpdate { player_id: String, money: i32 },
    #[serde(rename = "scoreboard_update")]
    ScoreboardUpdate { players: Vec<ScoreboardPlayer> },
    #[serde(rename = "round_start")]
    RoundStart { 
        round_number: i32, 
        buy_time: i32,
        orange_score: i32,
        red_score: i32,
        attacking_team: String,
    },
    #[serde(rename = "round_end")]
    RoundEnd { 
        winner: String, 
        reason: String,
        orange_score: i32,
        red_score: i32,
    },
    #[serde(rename = "build_phase_end")]
    BuildPhaseEnd { round_time: u64 },
    #[serde(rename = "bomb_planted")]
    BombPlanted { 
        player_id: String,
        timer: i32,
        position_x: f32,
        position_z: f32,
    },
    #[serde(rename = "bomb_defused")]
    BombDefused { 
        player_id: String,
    },
    #[serde(rename = "bomb_exploded")]
    BombExploded,
    #[serde(rename = "structure_placed")]
    StructurePlaced {
        player_id: String,
        structure_type: String,
        x: f32,
        y: f32,  // Will store rotation
        z: f32,
    },
    #[serde(rename = "give_bomb")]
    GiveBomb {
        player_id: String,
    },
    #[serde(rename = "bomb_dropped")]
    BombDropped {
        position_x: f32,
        position_y: f32,
        position_z: f32,
    },
    #[serde(rename = "bomb_picked_up")]
    BombPickedUp {
        player_id: String,
        player_name: String,
    },
    #[serde(rename = "match_end")]
    MatchEnd {
        winner: String,
    },
    #[serde(rename = "map_reset")]
    MapReset,
    // Authoritative bullet hole on a destructible wall — broadcast to every
    // client so they all render the same holes (and shots pass through them).
    #[serde(rename = "wall_hole")]
    WallHole { wall_x: f32, wall_z: f32, local_x: f32, world_y: f32, radius: f32 },
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ScoreboardPlayer {
    pub id: String,
    pub name: String,
    pub kills: i32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TeamPlayer {
    pub id: String,
    pub name: String,
}