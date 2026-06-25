use crate::game::Player;
use crate::game::spawn_system::SpawnSystem;
use crate::network::messages::ScoreboardPlayer;
use std::collections::HashMap;
use std::time::{Duration, Instant};

/// Seconds from plant until the bomb explodes.
pub const BOMB_TIMER_SECS: u64 = 40;

/// Seconds the attackers have during the Playing phase to plant the bomb. If the
/// round timer runs out BEFORE a plant, the defenders win (TimeUp). Once the bomb
/// is planted, this timer no longer matters — only BOMB_TIMER_SECS does.
pub const ROUND_TIMER_SECS: u64 = 100;

/// Round wins needed to win the whole match (first to 7).
pub const ROUND_WINS_TO_WIN_MATCH: i32 = 7;

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
    pub round_end_time: Option<Instant>,
    pub attacking_team: TeamColor, // Which team is attacking this half
    pub spawn_system: SpawnSystem, // Spawn system for team-based spawning
    pub structures: Vec<crate::game::collision::Wall>, // player-placed walls (for shot occlusion)
    pub match_end_announced: bool, // so the MatchEnd message is broadcast exactly once
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
            round_end_time: None,
            attacking_team: TeamColor::Red,  // Red team attacks first
            spawn_system: SpawnSystem::new(),
            structures: Vec::new(),
            match_end_announced: false,
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
            round_end_time: None,
            attacking_team: TeamColor::Red,  // Red team attacks first
            spawn_system: SpawnSystem::new(),
            structures: Vec::new(),
            match_end_announced: false,
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
    
    // Returns true if the sides were switched this round (halftime), so the
    // caller can broadcast a map reset to clear structures built in the first half.
    pub fn start_new_round(&mut self) -> bool {
        self.round_number += 1;
        log::info!("=== STARTING ROUND {} ===", self.round_number);

        // Switch sides at the start of round 7 (after each team has played
        // 6 rounds: 6 as attacker, 6 as defender). First to 7 wins.
        let mut sides_switched = false;
        if self.round_number == 7 {
            self.switch_sides();
            sides_switched = true;
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
            // Pick a random player from the attacking team. Seed from the
            // wall-clock nanoseconds so the carrier varies every round instead
            // of rotating deterministically with the round number.
            let seed = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.subsec_nanos() as usize)
                .unwrap_or(self.round_number as usize);
            let random_index = seed % attacking_players.len();
            let carrier_id = &attacking_players[random_index];
            self.bomb_carrier_id = Some(carrier_id.clone());
            if let Some(player) = self.players.get_mut(carrier_id) {
                player.has_bomb = true;
                log::info!("Player {} from {:?} team given the bomb for round {}", 
                    carrier_id, self.attacking_team, self.round_number);
            }
        }
        
        // Money is given in end_round() based on win/loss, not here
        // Only set starting money for the very first round
        log::info!("Round number check: {} == 1? {}", self.round_number, self.round_number == 1);
        if self.round_number == 1 {
            log::info!("FIRST ROUND - Resetting all players to $800");
            for player in self.players.values_mut() {
                let old_money = player.money;
                player.money = 800; // Pistol round starting money
                log::info!("Round 1: Player {} money reset from ${} to $800", player.id, old_money);
            }
        } else {
            // Log current money for all players at round start
            log::info!("NOT FIRST ROUND - Keeping player money from previous round");
            for player in self.players.values() {
                log::info!("Round {}: Player {} KEEPING ${} at round start", 
                    self.round_number, player.id, player.money);
            }
        }
        // Otherwise keep the money from end_round rewards + any kill rewards

        sides_switched
    }

    pub fn end_build_phase(&mut self) {
        if matches!(self.game_state, GameState::BuildPhase) {
            self.game_state = GameState::Playing;
            self.round_start_time = Some(Instant::now());
        }
    }
    
    pub fn check_build_phase_timeout(&mut self) -> bool {
        if let (GameState::BuildPhase, Some(start_time)) = (&self.game_state, self.build_phase_start) {
            if start_time.elapsed() >= Duration::from_secs(30) { // 30 seconds build phase
                self.end_build_phase();
                return true;
            }
        }
        false
    }
    
    pub fn plant_bomb(&mut self, player_id: &str, position_x: Option<f32>, position_z: Option<f32>) -> Option<(f32, f32)> {
        log::error!("=== CHECKING BOMB PLANT CONDITIONS ===");
        log::error!("Player: {}", player_id);
        log::error!("Game State: {:?} (needs to be Playing)", self.game_state);
        log::error!("Bomb Already Planted: {}", self.bomb_planted);
        log::error!("Current Bomb Carrier: {:?}", self.bomb_carrier_id);
        log::error!("Player is Carrier: {}", self.bomb_carrier_id.as_deref() == Some(player_id));
        log::error!("Game Mode: {:?}", self.game_mode);
        
        // In team mode, only the carrier can plant during Playing state (not BuildPhase)
        if matches!(self.game_state, GameState::Playing) && 
           !self.bomb_planted &&
           self.bomb_carrier_id.as_deref() == Some(player_id) {
            self.bomb_planted = true;
            self.bomb_plant_time = Some(Instant::now());
            self.bomb_carrier_id = None;
            
            // Use provided position if available, otherwise player position
            let position = if let (Some(x), Some(z)) = (position_x, position_z) {
                if let Some(player) = self.players.get_mut(player_id) {
                    player.has_bomb = false;
                }
                Some((x, z))
            } else if let Some(player) = self.players.get_mut(player_id) {
                player.has_bomb = false;
                Some((player.x, player.z))
            } else {
                Some((0.0, 0.0))
            };

            // Remember where the bomb was planted so defenders can be checked
            // for proximity when defusing.
            if let Some((x, z)) = position {
                self.bomb_position = Some((x, 1.0, z));
            }

            log::info!("✅ Player {} successfully planted the bomb at ({:?})", player_id, position);
            position
        } else {
            // Log specific failure reason
            if !matches!(self.game_state, GameState::Playing) {
                log::warn!("❌ PLANT FAILED: Game is in {:?} state, not Playing! Wait for build phase to end.", self.game_state);
            } else if self.bomb_planted {
                log::warn!("❌ PLANT FAILED: Bomb is already planted!");
            } else if self.bomb_carrier_id.as_deref() != Some(player_id) {
                log::warn!("❌ PLANT FAILED: Player {} is not the bomb carrier! Carrier is: {:?}", player_id, self.bomb_carrier_id);
            }
            None
        }
    }
    
    /// If the Playing phase has run past ROUND_TIMER_SECS and the bomb is NOT
    /// planted, time is up: the defenders win. Returns the winning (defending)
    /// team if the round just ended this way, else None. A planted bomb freezes
    /// this timer — the bomb explosion check takes over from there.
    pub fn check_round_timeout(&mut self) -> Option<TeamColor> {
        if self.bomb_planted {
            return None; // bomb is ticking — round timer no longer applies
        }
        if let (GameState::Playing, Some(start_time)) = (&self.game_state, self.round_start_time) {
            if start_time.elapsed() >= Duration::from_secs(ROUND_TIMER_SECS) {
                let defending_team = match self.attacking_team {
                    TeamColor::Orange => TeamColor::Red,
                    TeamColor::Red => TeamColor::Orange,
                };
                log::info!("⏱️ Round time up! {:?} (defenders) win — attackers failed to plant", defending_team);
                self.end_round(defending_team.clone(), RoundEndReason::TimeUp);
                return Some(defending_team);
            }
        }
        None
    }

    pub fn check_bomb_explosion(&mut self) -> bool {
        if let (true, Some(plant_time)) = (self.bomb_planted, self.bomb_plant_time) {
            let elapsed = plant_time.elapsed().as_secs();
            if elapsed >= BOMB_TIMER_SECS {
                log::info!("BOMB EXPLODING! Elapsed: {}s", elapsed);

                // Reset bomb state BEFORE ending round to prevent multiple calls
                self.bomb_planted = false;
                self.bomb_plant_time = None;

                // Determine winner based on attacking team (attackers win if bomb explodes)
                let winner = self.attacking_team.clone();
                self.end_round(winner, RoundEndReason::BombExploded);
                return true;
            } else {
                // Debug: show countdown
                if elapsed % 5 == 0 {
                    log::info!("Bomb countdown: {}s remaining", BOMB_TIMER_SECS - elapsed);
                }
            }
        }
        false
    }

    // Defending team defuses the planted bomb. The 5-second hold happens on the
    // client; this is the server-authoritative completion: validate that the
    // bomb is planted, the player is on the DEFENDING team, alive, and near the
    // bomb, then end the round in the defenders' favour. Returns the defending
    // (winning) team on success.
    pub fn defuse_bomb(&mut self, player_id: &str) -> Option<TeamColor> {
        if !self.bomb_planted {
            log::warn!("❌ DEFUSE FAILED: bomb is not planted");
            return None;
        }

        // The defending team is the opposite of the attacking team.
        let defending_team = match self.attacking_team {
            TeamColor::Orange => TeamColor::Red,
            TeamColor::Red => TeamColor::Orange,
        };

        let is_defender = match defending_team {
            TeamColor::Orange => self.orange_team.contains(&player_id.to_string()),
            TeamColor::Red => self.red_team.contains(&player_id.to_string()),
        };
        if !is_defender {
            log::warn!("❌ DEFUSE FAILED: player {} is not on the defending team", player_id);
            return None;
        }

        // Must be alive and near the bomb.
        let near_bomb = match (self.players.get(player_id), self.bomb_position) {
            (Some(p), Some(bomb_pos)) if p.alive => {
                let dist = ((p.x - bomb_pos.0).powi(2) + (p.z - bomb_pos.2).powi(2)).sqrt();
                dist < 10.0
            }
            _ => false,
        };
        if !near_bomb {
            log::warn!("❌ DEFUSE FAILED: player {} is dead or too far from the bomb", player_id);
            return None;
        }

        // Success: clear the bomb and award the round to the defenders.
        self.bomb_planted = false;
        self.bomb_plant_time = None;
        self.bomb_position = None;
        log::info!("✅ Player {} defused the bomb! {:?} (defenders) win the round", player_id, defending_team);
        self.end_round(defending_team.clone(), RoundEndReason::BombDefused);
        Some(defending_team)
    }

    pub fn end_round(&mut self, winner: TeamColor, reason: RoundEndReason) {
        log::info!("=== END_ROUND CALLED === Winner: {:?}, Reason: {:?}", winner, reason);
        log::info!("Current round: {}, Orange score: {}, Red score: {}", 
            self.round_number, self.orange_score, self.red_score);
        
        self.game_state = GameState::RoundEnd;
        self.round_end_time = Some(Instant::now()); // Track when round ended
        
        // Log all players' money BEFORE bonuses
        log::info!("Player money BEFORE round end bonuses:");
        for player in self.players.values() {
            log::info!("  Player {} (team: {:?}): ${}", player.id, player.team, player.money);
        }
        
        match winner {
            TeamColor::Orange => {
                self.orange_score += 1;
                // Give winner bonus
                for id in &self.orange_team {
                    if let Some(player) = self.players.get_mut(id) {
                        let old_money = player.money;
                        player.money = (player.money + 3000).min(16000); // Winner bonus
                        log::info!("WINNER: Player {} money: ${} -> ${} (+$3000 win bonus)", 
                            player.id, old_money, player.money);
                    }
                }
                // Give loser bonus
                for id in &self.red_team {
                    if let Some(player) = self.players.get_mut(id) {
                        let old_money = player.money;
                        player.money = (player.money + 1400).min(16000); // Loser bonus
                        log::info!("LOSER: Player {} money: ${} -> ${} (+$1400 loss bonus)", 
                            player.id, old_money, player.money);
                    }
                }
            },
            TeamColor::Red => {
                self.red_score += 1;
                // Give winner bonus
                for id in &self.red_team {
                    if let Some(player) = self.players.get_mut(id) {
                        let old_money = player.money;
                        player.money = (player.money + 3000).min(16000); // Winner bonus
                        log::info!("WINNER: Player {} money: ${} -> ${} (+$3000 win bonus)", 
                            player.id, old_money, player.money);
                    }
                }
                // Give loser bonus
                for id in &self.orange_team {
                    if let Some(player) = self.players.get_mut(id) {
                        let old_money = player.money;
                        player.money = (player.money + 1400).min(16000); // Loser bonus
                        log::info!("LOSER: Player {} money: ${} -> ${} (+$1400 loss bonus)", 
                            player.id, old_money, player.money);
                    }
                }
            }
        }
        
        // Check for match end (first to ROUND_WINS_TO_WIN_MATCH round wins)
        if self.orange_score >= ROUND_WINS_TO_WIN_MATCH || self.red_score >= ROUND_WINS_TO_WIN_MATCH {
            self.game_state = GameState::MatchEnd;
        }
        
        // Log all players' money AFTER bonuses
        log::info!("Player money AFTER round end bonuses:");
        for player in self.players.values() {
            log::info!("  Player {} (team: {:?}): ${} (should be sent to client)", 
                player.id, player.team, player.money);
        }
        log::info!("=== END_ROUND COMPLETE ===");
    }
    
    // Returns (restarted, sides_switched). sides_switched implies a halftime
    // map reset should be broadcast.
    pub fn check_round_restart(&mut self) -> (bool, bool) {
        // After round ends, wait 5 seconds then start new round
        if matches!(self.game_state, GameState::RoundEnd) {
            if let Some(end_time) = self.round_end_time {
                if end_time.elapsed() >= Duration::from_secs(5) {
                    // Don't restart if match is over
                    if !matches!(self.game_state, GameState::MatchEnd) {
                        let switched = self.start_new_round();
                        return (true, switched);
                    }
                }
            }
        }
        (false, false)
    }
    
    /// If the match has just ended (a team reached 7 round wins → game_state is
    /// MatchEnd) and we haven't announced it yet, return the winning team ("orange"
    /// or "red") ONCE and mark it announced so the MatchEnd message is broadcast a
    /// single time. Returns None otherwise.
    pub fn take_match_end_winner(&mut self) -> Option<&'static str> {
        if matches!(self.game_state, GameState::MatchEnd) && !self.match_end_announced {
            self.match_end_announced = true;
            let winner = if self.orange_score >= ROUND_WINS_TO_WIN_MATCH { "orange" } else { "red" };
            return Some(winner);
        }
        None
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

        let winner = if !orange_alive && red_alive {
            TeamColor::Red
        } else if orange_alive && !red_alive {
            TeamColor::Orange
        } else {
            return None;
        };

        // POST-PLANT RULE (CS/Valorant): once the bomb is planted, killing all the
        // ATTACKERS does NOT win the round — the bomb is ticking and the defenders
        // must DEFUSE it (or it explodes and attackers win even while dead). So if
        // the bomb is planted and the would-be winner is the DEFENDING team, the
        // round does not end on elimination. (If the DEFENDERS are wiped post-plant,
        // attackers still win immediately — no one is left to defuse.)
        if self.bomb_planted {
            let defending_team = match self.attacking_team {
                TeamColor::Orange => TeamColor::Red,
                TeamColor::Red => TeamColor::Orange,
            };
            // Compare by discriminant (TeamColor isn't PartialEq).
            let winner_is_defender = matches!(
                (&winner, &defending_team),
                (TeamColor::Orange, TeamColor::Orange) | (TeamColor::Red, TeamColor::Red)
            );
            if winner_is_defender {
                return None; // defenders must defuse, not just kill
            }
        }

        Some(winner)
    }

    pub fn get_spawn_position(&self, player_id: &str) -> (f32, f32, f32) {
        match self.game_mode {
            GameMode::Deathmatch => (0.0, 10.0, 0.0),
            GameMode::TeamVsTeam => {
                // Spawn is based on the team's CURRENT role (attacker/defender),
                // not its color. This way when sides switch at halftime the
                // players actually swap to the other side of the map.
                let pid = player_id.to_string();
                if self.orange_team.contains(&pid) {
                    let team_index = self.orange_team.iter().position(|id| id == player_id).unwrap_or(0);
                    let role = match self.attacking_team {
                        TeamColor::Orange => "attacker",
                        TeamColor::Red => "defender",
                    };
                    self.spawn_system.get_team_spawn_position(role, team_index)
                } else if self.red_team.contains(&pid) {
                    let team_index = self.red_team.iter().position(|id| id == player_id).unwrap_or(0);
                    let role = match self.attacking_team {
                        TeamColor::Red => "attacker",
                        TeamColor::Orange => "defender",
                    };
                    self.spawn_system.get_team_spawn_position(role, team_index)
                } else {
                    (0.0, 10.0, 0.0)
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