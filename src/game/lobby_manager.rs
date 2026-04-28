use crate::game::lobby::{Lobby, GameMode, TeamColor, GameState};
use crate::game::Player;
use std::collections::HashMap;
use std::sync::Mutex;
use uuid::Uuid;

pub struct LobbyManager {
    lobbies: Mutex<HashMap<String, Lobby>>,
    deathmatch_lobby_id: Mutex<Option<String>>,
    team_selection_lobby_id: Mutex<Option<String>>,
}

impl LobbyManager {
    pub fn new() -> Self {
        let manager = Self {
            lobbies: Mutex::new(HashMap::new()),
            deathmatch_lobby_id: Mutex::new(None),
            team_selection_lobby_id: Mutex::new(None),
        };
        
        manager.ensure_deathmatch_lobby();
        manager
    }

    fn ensure_deathmatch_lobby(&self) {
        let mut lobbies = self.lobbies.lock().unwrap();
        let mut dm_id = self.deathmatch_lobby_id.lock().unwrap();
        
        if dm_id.is_none() {
            let lobby_id = Uuid::new_v4().to_string();
            let lobby = Lobby::new_deathmatch(lobby_id.clone());
            lobbies.insert(lobby_id.clone(), lobby);
            *dm_id = Some(lobby_id);
        }
    }

    pub fn get_or_create_team_selection_lobby(&self) -> String {
        let mut lobbies = self.lobbies.lock().unwrap();
        let mut team_lobby_id = self.team_selection_lobby_id.lock().unwrap();
        
        // If there's already a team selection lobby with space, use it
        if let Some(id) = &*team_lobby_id {
            if let Some(lobby) = lobbies.get(id) {
                if lobby.players.len() < 10 && !lobby.game_started {
                    return id.clone();
                }
            }
        }
        
        // Create a new team selection lobby
        let lobby_id = Uuid::new_v4().to_string();
        let lobby = Lobby::new_team_vs_team(lobby_id.clone());
        lobbies.insert(lobby_id.clone(), lobby);
        *team_lobby_id = Some(lobby_id.clone());
        lobby_id
    }
    
    pub fn create_team_game_from_selection(&self, selection_lobby_id: &str) -> Option<String> {
        // Create a new game lobby from the team selection
        let game_lobby_id = Uuid::new_v4().to_string();
        let game_lobby = Lobby::new_team_vs_team(game_lobby_id.clone());
        
        let mut lobbies = self.lobbies.lock().unwrap();
        lobbies.insert(game_lobby_id.clone(), game_lobby);
        
        // Clear the selection lobby for next group
        if let Some(selection_lobby) = lobbies.get_mut(selection_lobby_id) {
            selection_lobby.game_started = true;
        }
        
        Some(game_lobby_id)
    }

    pub fn get_deathmatch_lobby_id(&self) -> String {
        let dm_id = self.deathmatch_lobby_id.lock().unwrap();
        dm_id.clone().unwrap()
    }

    pub fn join_lobby(&self, lobby_id: &str, player: Player) -> bool {
        let mut lobbies = self.lobbies.lock().unwrap();
        if let Some(lobby) = lobbies.get_mut(lobby_id) {
            lobby.add_player(player)
        } else {
            false
        }
    }

    pub fn leave_lobby(&self, lobby_id: &str, player_id: &str) {
        let mut lobbies = self.lobbies.lock().unwrap();
        if let Some(lobby) = lobbies.get_mut(lobby_id) {
            lobby.remove_player(player_id);
            
            if matches!(lobby.game_mode, GameMode::TeamVsTeam) && lobby.players.is_empty() {
                lobbies.remove(lobby_id);
            }
        }
    }

    pub fn join_team(&self, lobby_id: &str, player_id: &str, team: TeamColor) -> bool {
        let mut lobbies = self.lobbies.lock().unwrap();
        if let Some(lobby) = lobbies.get_mut(lobby_id) {
            lobby.join_team(player_id, team)
        } else {
            false
        }
    }

    pub fn start_team_game(&self, lobby_id: &str) -> bool {
        let mut lobbies = self.lobbies.lock().unwrap();
        if let Some(lobby) = lobbies.get_mut(lobby_id) {
            lobby.start_game()
        } else {
            false
        }
    }

    pub fn get_lobby_players(&self, lobby_id: &str) -> Vec<Player> {
        let lobbies = self.lobbies.lock().unwrap();
        if let Some(lobby) = lobbies.get(lobby_id) {
            lobby.players.values().cloned().collect()
        } else {
            Vec::new()
        }
    }

    pub fn get_team_data(&self, lobby_id: &str) -> Option<(Vec<(String, String)>, Vec<(String, String)>)> {
        let lobbies = self.lobbies.lock().unwrap();
        lobbies.get(lobby_id).map(|lobby| lobby.get_team_data())
    }

    pub fn update_player_position(&self, lobby_id: &str, player_id: &str, x: f32, y: f32, z: f32, rotation_x: f32, rotation_y: f32) {
        let mut lobbies = self.lobbies.lock().unwrap();
        if let Some(lobby) = lobbies.get_mut(lobby_id) {
            lobby.update_player_position(player_id, x, y, z, rotation_x, rotation_y);
        }
    }

    pub fn damage_player(&self, lobby_id: &str, player_id: &str, damage: i32) -> Option<(bool, i32, i32)> {
        let mut lobbies = self.lobbies.lock().unwrap();
        if let Some(lobby) = lobbies.get_mut(lobby_id) {
            lobby.damage_player(player_id, damage)
        } else {
            None
        }
    }

    pub fn respawn_player(&self, lobby_id: &str, player_id: &str) -> Option<Player> {
        let mut lobbies = self.lobbies.lock().unwrap();
        if let Some(lobby) = lobbies.get_mut(lobby_id) {
            lobby.respawn_player(player_id)
        } else {
            None
        }
    }

    pub fn add_kill_to_player(&self, lobby_id: &str, player_id: &str) -> bool {
        let mut lobbies = self.lobbies.lock().unwrap();
        if let Some(lobby) = lobbies.get_mut(lobby_id) {
            lobby.add_kill_to_player(player_id)
        } else {
            false
        }
    }

    pub fn update_shields(&self, lobby_id: &str) -> Vec<(String, i32)> {
        let mut lobbies = self.lobbies.lock().unwrap();
        if let Some(lobby) = lobbies.get_mut(lobby_id) {
            lobby.update_shields()
        } else {
            Vec::new()
        }
    }

    pub fn get_lobby_scoreboard(&self, lobby_id: &str) -> Vec<crate::network::messages::ScoreboardPlayer> {
        let lobbies = self.lobbies.lock().unwrap();
        if let Some(lobby) = lobbies.get(lobby_id) {
            lobby.get_scoreboard_data()
        } else {
            Vec::new()
        }
    }

    pub fn cleanup_empty_team_lobbies(&self) {
        let mut lobbies = self.lobbies.lock().unwrap();
        let dm_id = self.deathmatch_lobby_id.lock().unwrap();
        
        lobbies.retain(|id, lobby| {
            if let Some(dm_lobby_id) = &*dm_id {
                if id == dm_lobby_id {
                    return true;
                }
            }
            
            if matches!(lobby.game_mode, GameMode::TeamVsTeam) {
                !lobby.players.is_empty()
            } else {
                true
            }
        });
    }
    
    pub fn plant_bomb(&self, lobby_id: &str, player_id: &str) -> bool {
        let mut lobbies = self.lobbies.lock().unwrap();
        if let Some(lobby) = lobbies.get_mut(lobby_id) {
            lobby.plant_bomb(player_id)
        } else {
            false
        }
    }
    
    pub fn get_player(&self, lobby_id: &str, player_id: &str) -> Option<Player> {
        let lobbies = self.lobbies.lock().unwrap();
        if let Some(lobby) = lobbies.get(lobby_id) {
            lobby.players.get(player_id).cloned()
        } else {
            None
        }
    }
    
    pub fn get_bomb_carrier(&self, lobby_id: &str) -> Option<String> {
        let lobbies = self.lobbies.lock().unwrap();
        if let Some(lobby) = lobbies.get(lobby_id) {
            lobby.bomb_carrier_id.clone()
        } else {
            None
        }
    }
    
    pub fn can_respawn(&self, lobby_id: &str) -> bool {
        let lobbies = self.lobbies.lock().unwrap();
        if let Some(lobby) = lobbies.get(lobby_id) {
            // Allow respawn only in deathmatch or during round end/build phase
            matches!(lobby.game_mode, GameMode::Deathmatch) ||
            matches!(lobby.game_state, GameState::RoundEnd | GameState::BuildPhase | GameState::WaitingForPlayers)
        } else {
            false
        }
    }
    
    pub fn check_team_elimination(&self, lobby_id: &str) -> Option<TeamColor> {
        let mut lobbies = self.lobbies.lock().unwrap();
        if let Some(lobby) = lobbies.get_mut(lobby_id) {
            lobby.check_team_elimination()
        } else {
            None
        }
    }
    
    pub fn end_round(&self, lobby_id: &str, winner: TeamColor, reason: &str) {
        let mut lobbies = self.lobbies.lock().unwrap();
        if let Some(lobby) = lobbies.get_mut(lobby_id) {
            let round_reason = match reason {
                "Team Eliminated" => crate::game::lobby::RoundEndReason::TeamEliminated,
                "Bomb Exploded" => crate::game::lobby::RoundEndReason::BombExploded,
                "Bomb Defused" => crate::game::lobby::RoundEndReason::BombDefused,
                _ => crate::game::lobby::RoundEndReason::TimeUp,
            };
            lobby.end_round(winner, round_reason);
        }
    }
    
    pub fn get_team_scores(&self, lobby_id: &str) -> (i32, i32) {
        let lobbies = self.lobbies.lock().unwrap();
        if let Some(lobby) = lobbies.get(lobby_id) {
            (lobby.orange_score, lobby.red_score)
        } else {
            (0, 0)
        }
    }
    
    pub fn start_new_round(&self, lobby_id: &str) {
        let mut lobbies = self.lobbies.lock().unwrap();
        if let Some(lobby) = lobbies.get_mut(lobby_id) {
            lobby.start_new_round();
        }
    }
    
    pub fn get_round_number(&self, lobby_id: &str) -> i32 {
        let lobbies = self.lobbies.lock().unwrap();
        if let Some(lobby) = lobbies.get(lobby_id) {
            lobby.round_number
        } else {
            0
        }
    }
    
    pub fn check_all_build_phase_timeouts(&self, broadcaster: &crate::network::MessageBroadcaster) {
        let mut lobbies = self.lobbies.lock().unwrap();
        let lobby_ids: Vec<String> = lobbies.keys().cloned().collect();
        
        for lobby_id in lobby_ids {
            if let Some(lobby) = lobbies.get_mut(&lobby_id) {
                if lobby.check_build_phase_timeout() {
                    // Build phase ended, notify all players in lobby
                    let msg = crate::network::messages::ServerMessage::BuildPhaseEnd;
                    broadcaster.broadcast_to_lobby(&lobby_id, &msg, None);
                    log::info!("Build phase ended for lobby {}", lobby_id);
                }
            }
        }
    }
    
    pub fn check_all_bomb_explosions(&self, broadcaster: &crate::network::MessageBroadcaster) {
        let mut lobbies = self.lobbies.lock().unwrap();
        let lobby_ids: Vec<String> = lobbies.keys().cloned().collect();
        
        for lobby_id in lobby_ids {
            if let Some(lobby) = lobbies.get_mut(&lobby_id) {
                if lobby.check_bomb_explosion() {
                    // Bomb exploded, notify all players in lobby
                    let (orange_score, red_score) = (lobby.orange_score, lobby.red_score);
                    // The attacking team wins when bomb explodes
                    let winner = match lobby.attacking_team {
                        crate::game::TeamColor::Orange => "orange",
                        crate::game::TeamColor::Red => "red",
                    };
                    let msg = crate::network::messages::ServerMessage::RoundEnd {
                        winner: winner.to_string(),
                        reason: "Bomb Exploded".to_string(),
                        orange_score,
                        red_score,
                    };
                    broadcaster.broadcast_to_lobby(&lobby_id, &msg, None);
                    log::info!("Bomb exploded in lobby {}, {} team wins", lobby_id, winner);
                }
            }
        }
    }
    
    pub fn drop_bomb(&self, lobby_id: &str, player_id: &str) -> Option<(f32, f32, f32)> {
        let mut lobbies = self.lobbies.lock().unwrap();
        if let Some(lobby) = lobbies.get_mut(lobby_id) {
            lobby.drop_bomb(player_id)
        } else {
            None
        }
    }
    
    pub fn pickup_bomb(&self, lobby_id: &str, player_id: &str) -> Option<String> {
        let mut lobbies = self.lobbies.lock().unwrap();
        if let Some(lobby) = lobbies.get_mut(lobby_id) {
            if lobby.pickup_bomb(player_id) {
                // Return player name
                lobby.players.get(player_id).map(|p| p.name.clone())
            } else {
                None
            }
        } else {
            None
        }
    }
    
    pub fn get_attacking_team(&self, lobby_id: &str) -> Option<crate::game::TeamColor> {
        let lobbies = self.lobbies.lock().unwrap();
        lobbies.get(lobby_id).map(|lobby| lobby.attacking_team.clone())
    }
}