use crate::game::Player;
use crate::network::messages::ScoreboardPlayer;
use std::collections::HashMap;
use std::time::{Duration, Instant};

#[derive(Debug, Clone)]
pub enum GameMode {
    Deathmatch,
    TeamVsTeam,
}

#[derive(Debug, Clone)]
pub enum TeamColor {
    Orange,
    Red,
}

#[derive(Debug, Clone)]
pub enum GameState {
    WaitingForPlayers,
    BuildPhase,  // Build mode phase where players can place structures
    Playing,
    RoundEnd,
    MatchEnd,
}

#[derive(Debug, Clone)]
pub enum RoundEndReason {
    BombExploded,
    BombDefused,
    TeamEliminated,
    TimeUp,
}

#[derive(Debug)]
pub struct Lobby {
    pub id: String,
    pub game_mode: GameMode,
    pub players: HashMap<String, Player>,
    pub max_players: usize,
    pub created_at: Instant,
    pub game_started: bool,
    pub orange_team: Vec<String>,
    pub red_team: Vec<String>,
    pub game_state: GameState,
    pub round_number: i32,
    pub orange_score: i32,
    pub red_score: i32,
    pub bomb_planted: bool,
    pub bomb_plant_time: Option<Instant>,
    pub bomb_carrier_id: Option<String>,
    pub bomb_dropped: bool,
    pub bomb_position: Option<(f32, f32, f32)>, // Position when dropped
    pub build_phase_start: Option<Instant>,
    pub round_start_time: Option<Instant>,
    pub attacking_team: TeamColor, // Which team is attacking this half
}

impl Lobby {
    pub fn new_deathmatch(id: String) -> Self {
        Self {
            id,
            game_mode: GameMode::Deathmatch,
            players: HashMap::new(),
            max_players: 20,
            created_at: Instant::now(),
            game_started: true,
            orange_team: Vec::new(),
            red_team: Vec::new(),
            game_state: GameState::Playing,
            round_number: 0,
            orange_score: 0,
            red_score: 0,
            bomb_planted: false,
            bomb_plant_time: None,
            bomb_carrier_id: None,
            bomb_dropped: false,
            bomb_position: None,
            build_phase_start: None,
            round_start_time: None,
            attacking_team: TeamColor::Red,  // Red team attacks first
        }
    }

    pub fn new_team_vs_team(id: String) -> Self {
        Self {
            id,
            game_mode: GameMode::TeamVsTeam,
            players: HashMap::new(),
            max_players: 10,
            created_at: Instant::now(),
            game_started: false,
            orange_team: Vec::new(),
            red_team: Vec::new(),
            game_state: GameState::WaitingForPlayers,
            round_number: 0,
            orange_score: 0,
            red_score: 0,
            bomb_planted: false,
            bomb_plant_time: None,
            bomb_carrier_id: None,
            bomb_dropped: false,
            bomb_position: None,
            build_phase_start: None,
            round_start_time: None,
            attacking_team: TeamColor::Red,  // Red team attacks first
        }
    }

    pub fn add_player(&mut self, player: Player) -> bool {
        if self.players.len() >= self.max_players {
            return false;
        }
        self.players.insert(player.id.clone(), player);
        true
    }

    pub fn remove_player(&mut self, player_id: &str) {
        self.players.remove(player_id);
        self.orange_team.retain(|id| id != player_id);
        self.red_team.retain(|id| id != player_id);
    }

    pub fn join_team(&mut self, player_id: &str, team: TeamColor) -> bool {
        if !self.players.contains_key(player_id) {
            return false;
        }

        self.orange_team.retain(|id| id != player_id);
        self.red_team.retain(|id| id != player_id);

        match team {
            TeamColor::Orange => {
                if self.orange_team.len() < 5 {
                    self.orange_team.push(player_id.to_string());
                    if let Some(player) = self.players.get_mut(player_id) {
                        player.team = Some("orange".to_string());
                    }
                    true
                } else {
                    false
                }
            }
            TeamColor::Red => {
                if self.red_team.len() < 5 {
                    self.red_team.push(player_id.to_string());
                    if let Some(player) = self.players.get_mut(player_id) {
                        player.team = Some("red".to_string());
                    }
                    true
                } else {
                    false
                }
            }
        }
    }

    pub fn can_start_team_game(&self) -> bool {
        matches!(self.game_mode, GameMode::TeamVsTeam) && 
        self.orange_team.len() >= 1 && 
        self.red_team.len() >= 1
    }

    pub fn start_game(&mut self) -> bool {
        if self.can_start_team_game() || matches!(self.game_mode, GameMode::Deathmatch) {
            self.game_started = true;
            if matches!(self.game_mode, GameMode::TeamVsTeam) {
                // Collect spawn positions first to avoid borrow issues
                let spawn_positions: Vec<(String, (f32, f32, f32))> = self.players.keys()
                    .map(|id| (id.clone(), self.get_spawn_position(id)))
                    .collect();
                
                // Now update player positions
                for (player_id, spawn_pos) in spawn_positions {
                    if let Some(player) = self.players.get_mut(&player_id) {
                        player.x = spawn_pos.0;
                        player.y = spawn_pos.1;
                        player.z = spawn_pos.2;
                    }
                }
                self.start_new_round();
            }
            true
        } else {
            false
        }
    }
    
    pub fn start_new_round(&mut self) {
        self.round_number += 1;
        
        // Switch sides after round 6 (for testing, normally 15 in CS:GO)
        if self.round_number == 7 {
            self.switch_sides();
            log::info!("Switching sides at round 7!");
        }
        
        self.game_state = GameState::BuildPhase;
        self.build_phase_start = Some(Instant::now());
        self.bomb_planted = false;
        self.bomb_plant_time = None;
        
        // Collect spawn positions first to avoid borrow issues
        let spawn_positions: Vec<(String, (f32, f32, f32))> = self.players.keys()
            .map(|id| (id.clone(), self.get_spawn_position(id)))
            .collect();
        
        // Reset all players for new round and move them to spawn positions
        for (player_id, spawn_pos) in spawn_positions {
            if let Some(player) = self.players.get_mut(&player_id) {
                player.x = spawn_pos.0;
                player.y = spawn_pos.1;
                player.z = spawn_pos.2;
                player.alive = true;
                player.health = 100;
                player.shield = 100;
                player.has_bomb = false;
            }
        }
        
        // Give bomb to ONE random player from the ATTACKING team only
        let attacking_players = match self.attacking_team {
            TeamColor::Orange => &self.orange_team,
            TeamColor::Red => &self.red_team,
        };
        
        if !attacking_players.is_empty() {
            // Pick a random player from attacking team
            let random_index = (self.round_number as usize) % attacking_players.len();
            let carrier_id = &attacking_players[random_index];
            self.bomb_carrier_id = Some(carrier_id.clone());
            if let Some(player) = self.players.get_mut(carrier_id) {
                player.has_bomb = true;
                log::info!("Player {} from {:?} team given the bomb for round {}", 
                    carrier_id, self.attacking_team, self.round_number);
            }
        }
        
        // Give money based on round outcome (simplified)
        for player in self.players.values_mut() {
            if self.round_number == 1 {
                player.money = 800; // Pistol round
            } else {
                player.money = (player.money + 3250).min(16000); // Loss bonus for now
            }
        }
    }
    
    pub fn end_build_phase(&mut self) {
        if matches!(self.game_state, GameState::BuildPhase) {
            self.game_state = GameState::Playing;
            self.round_start_time = Some(Instant::now());
        }
    }
    
    pub fn check_build_phase_timeout(&mut self) -> bool {
        if let (GameState::BuildPhase, Some(start_time)) = (&self.game_state, self.build_phase_start) {
            if start_time.elapsed() >= Duration::from_secs(15) {
                self.end_build_phase();
                return true;
            }
        }
        false
    }
    
    pub fn plant_bomb(&mut self, player_id: &str) -> bool {
        if matches!(self.game_state, GameState::Playing) && 
           !self.bomb_planted &&
           self.bomb_carrier_id.as_deref() == Some(player_id) {
            self.bomb_planted = true;
            self.bomb_plant_time = Some(Instant::now());
            self.bomb_carrier_id = None;
            if let Some(player) = self.players.get_mut(player_id) {
                player.has_bomb = false;
            }
            true
        } else {
            false
        }
    }
    
    pub fn check_bomb_explosion(&mut self) -> bool {
        if let (true, Some(plant_time)) = (self.bomb_planted, self.bomb_plant_time) {
            if plant_time.elapsed() >= Duration::from_secs(45) { // CS:GO bomb timer
                // Determine winner based on attacking team (attackers win if bomb explodes)
                let winner = self.attacking_team.clone();
                self.end_round(winner, RoundEndReason::BombExploded);
                return true;
            }
        }
        false
    }
    
    pub fn end_round(&mut self, winner: TeamColor, _reason: RoundEndReason) {
        self.game_state = GameState::RoundEnd;
        
        match winner {
            TeamColor::Orange => {
                self.orange_score += 1;
                // Give winner bonus
                for id in &self.orange_team {
                    if let Some(player) = self.players.get_mut(id) {
                        player.money = (player.money + 3000).min(16000); // Winner bonus
                    }
                }
                // Give loser bonus
                for id in &self.red_team {
                    if let Some(player) = self.players.get_mut(id) {
                        player.money = (player.money + 1400).min(16000); // Loser bonus
                    }
                }
            },
            TeamColor::Red => {
                self.red_score += 1;
                // Give winner bonus
                for id in &self.red_team {
                    if let Some(player) = self.players.get_mut(id) {
                        player.money = (player.money + 3000).min(16000); // Winner bonus
                    }
                }
                // Give loser bonus
                for id in &self.orange_team {
                    if let Some(player) = self.players.get_mut(id) {
                        player.money = (player.money + 1400).min(16000); // Loser bonus
                    }
                }
            }
        }
        
        // Check for match end (first to 16)
        if self.orange_score >= 16 || self.red_score >= 16 {
            self.game_state = GameState::MatchEnd;
        }
    }
    
    pub fn switch_sides(&mut self) {
        // Switch which team is attacking
        self.attacking_team = match self.attacking_team {
            TeamColor::Orange => TeamColor::Red,
            TeamColor::Red => TeamColor::Orange,
        };
        
        log::info!("Sides switched! {:?} team is now attacking", self.attacking_team);
    }
    
    pub fn check_team_elimination(&mut self) -> Option<TeamColor> {
        // Check elimination during both BuildPhase and Playing (not during RoundEnd or WaitingForPlayers)
        if !matches!(self.game_state, GameState::BuildPhase | GameState::Playing) {
            return None;
        }
        
        let orange_alive = self.orange_team.iter()
            .any(|id| self.players.get(id).map_or(false, |p| p.alive));
        let red_alive = self.red_team.iter()
            .any(|id| self.players.get(id).map_or(false, |p| p.alive));
        
        if !orange_alive && red_alive {
            Some(TeamColor::Red)
        } else if orange_alive && !red_alive {
            Some(TeamColor::Orange)
        } else {
            None
        }
    }

    pub fn get_spawn_position(&self, player_id: &str) -> (f32, f32, f32) {
        match self.game_mode {
            GameMode::Deathmatch => (0.0, 5.0, 0.0),
            GameMode::TeamVsTeam => {
                if self.orange_team.contains(&player_id.to_string()) {
                    (-300.0, 5.0, 0.0)  // Move spawn more inside
                } else if self.red_team.contains(&player_id.to_string()) {
                    (300.0, 5.0, 0.0)   // Move spawn more inside
                } else {
                    (0.0, 5.0, 0.0)
                }
            }
        }
    }

    pub fn update_player_position(&mut self, player_id: &str, x: f32, y: f32, z: f32, rotation_x: f32, rotation_y: f32) {
        if let Some(player) = self.players.get_mut(player_id) {
            player.update_position(x, y, z, rotation_x, rotation_y);
        }
    }

    pub fn damage_player(&mut self, player_id: &str, damage: i32) -> Option<(bool, i32, i32)> {
        if let Some(player) = self.players.get_mut(player_id) {
            if !player.alive {
                return None;
            }
            let died = player.take_damage(damage);
            Some((died, player.health, player.shield))
        } else {
            None
        }
    }

    pub fn respawn_player(&mut self, player_id: &str) -> Option<Player> {
        let spawn_pos = self.get_spawn_position(player_id);
        if let Some(player) = self.players.get_mut(player_id) {
            player.respawn(spawn_pos);
            Some(player.clone())
        } else {
            None
        }
    }

    pub fn add_kill_to_player(&mut self, player_id: &str) -> bool {
        if let Some(player) = self.players.get_mut(player_id) {
            player.add_kill();
            true
        } else {
            false
        }
    }

    pub fn update_shields(&mut self) -> Vec<(String, i32)> {
        let mut shield_updates = Vec::new();
        
        for player in self.players.values_mut() {
            if player.update_shield_regen() {
                shield_updates.push((player.id.clone(), player.shield));
            }
        }
        
        shield_updates
    }

    pub fn get_scoreboard_data(&self) -> Vec<ScoreboardPlayer> {
        let mut scoreboard_players: Vec<ScoreboardPlayer> = self.players
            .values()
            .map(|player| ScoreboardPlayer {
                id: player.id.clone(),
                name: player.name.clone(),
                kills: player.kills,
            })
            .collect();
        
        scoreboard_players.sort_by(|a, b| b.kills.cmp(&a.kills));
        scoreboard_players
    }

    pub fn get_team_data(&self) -> (Vec<(String, String)>, Vec<(String, String)>) {
        let orange_players: Vec<(String, String)> = self.orange_team
            .iter()
            .filter_map(|id| {
                self.players.get(id).map(|p| (id.clone(), p.name.clone()))
            })
            .collect();

        let red_players: Vec<(String, String)> = self.red_team
            .iter()
            .filter_map(|id| {
                self.players.get(id).map(|p| (id.clone(), p.name.clone()))
            })
            .collect();

        (orange_players, red_players)
    }
    
    pub fn drop_bomb(&mut self, player_id: &str) -> Option<(f32, f32, f32)> {
        // Check if player has the bomb
        if let Some(carrier_id) = &self.bomb_carrier_id {
            if carrier_id == player_id {
                if let Some(player) = self.players.get_mut(player_id) {
                    // Always drop at ground level (y = 1)
                    let position = (player.x, 1.0, player.z);
                    player.has_bomb = false;
                    self.bomb_carrier_id = None;
                    self.bomb_dropped = true;
                    self.bomb_position = Some(position);
                    return Some(position);
                }
            }
        }
        None
    }
    
    pub fn pickup_bomb(&mut self, player_id: &str) -> bool {
        // Check if bomb is dropped and player is on attacking team
        if self.bomb_dropped {
            if let Some(player) = self.players.get_mut(player_id) {
                // Check if player is on attacking team
                let is_attacker = match self.attacking_team {
                    TeamColor::Red => self.red_team.contains(&player_id.to_string()),
                    TeamColor::Orange => self.orange_team.contains(&player_id.to_string()),
                };
                
                if is_attacker {
                    // Check if player is near the bomb
                    if let Some(bomb_pos) = self.bomb_position {
                        let distance = ((player.x - bomb_pos.0).powi(2) + 
                                      (player.z - bomb_pos.2).powi(2)).sqrt();
                        
                        if distance < 5.0 { // Within 5 units
                            player.has_bomb = true;
                            self.bomb_carrier_id = Some(player_id.to_string());
                            self.bomb_dropped = false;
                            self.bomb_position = None;
                            log::info!("Player {} (attacker) picked up bomb", player_id);
                            return true;
                        }
                    }
                } else {
                    log::info!("Player {} (defender) cannot pick up bomb", player_id);
                }
            }
        }
        false
    }
    
    pub fn handle_player_death(&mut self, player_id: &str) -> Option<(f32, f32, f32)> {
        // If dead player had bomb, drop it
        if let Some(carrier_id) = &self.bomb_carrier_id {
            if carrier_id == player_id {
                return self.drop_bomb(player_id);
            }
        }
        None
    }
}