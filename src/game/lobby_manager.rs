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

        if let Some(id) = &*team_lobby_id {
            if let Some(lobby) = lobbies.get(id) {
                if lobby.players.len() < 10 && !lobby.game_started {
                    return id.clone();
                }
            }
        }

        let lobby_id = Uuid::new_v4().to_string();
        let lobby = Lobby::new_team_vs_team(lobby_id.clone());
        lobbies.insert(lobby_id.clone(), lobby);
        *team_lobby_id = Some(lobby_id.clone());
        lobby_id
    }

    pub fn create_team_game_from_selection(&self, selection_lobby_id: &str) -> Option<String> {

        let game_lobby_id = Uuid::new_v4().to_string();
        let game_lobby = Lobby::new_team_vs_team(game_lobby_id.clone());

        let mut lobbies = self.lobbies.lock().unwrap();
        lobbies.insert(game_lobby_id.clone(), game_lobby);

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

    pub fn add_structure(&self, lobby_id: &str, structure_type: &str, x: f32, z: f32, rot: f32) {
        use crate::game::collision::Wall;

        let (height, destructible) = match structure_type {
            "large" => (20.0, false),
            "destructible" => (20.0, true),
            _ => (10.0, false),
        };
        let wall = Wall {
            cx: x,
            cz: z,
            hw: 10.0,
            hd: 1.0,
            rot,
            y_min: 0.0,
            y_max: height,
            destructible,
            holes: Vec::new(),
        };
        let mut lobbies = self.lobbies.lock().unwrap();
        if let Some(lobby) = lobbies.get_mut(lobby_id) {
            lobby.structures.push(wall);
        }
    }

    pub fn add_wall_hole(&self, lobby_id: &str, wall_x: f32, wall_z: f32, local_x: f32, world_y: f32, radius: f32) -> bool {
        use crate::game::collision::Hole;
        const MAX_HOLES: usize = 2;
        const MATCH_DIST2: f32 = 4.0;
        let mut lobbies = self.lobbies.lock().unwrap();
        let lobby = match lobbies.get_mut(lobby_id) { Some(l) => l, None => return false };

        let mut best: Option<(usize, f32)> = None;
        for (i, w) in lobby.structures.iter().enumerate() {
            if !w.destructible { continue; }
            let d2 = (w.cx - wall_x).powi(2) + (w.cz - wall_z).powi(2);
            if d2 <= MATCH_DIST2 && best.map_or(true, |(_, bd)| d2 < bd) {
                best = Some((i, d2));
            }
        }
        if let Some((i, _)) = best {
            let w = &mut lobby.structures[i];
            if w.holes.len() >= MAX_HOLES { return false; }
            w.holes.push(Hole { lx: local_x, ly: world_y, radius });
            return true;
        }
        false
    }

    pub fn clear_structures(&self, lobby_id: &str) {
        let mut lobbies = self.lobbies.lock().unwrap();
        if let Some(lobby) = lobbies.get_mut(lobby_id) {
            lobby.structures.clear();
        }
    }

    pub fn shot_is_blocked(&self, lobby_id: &str, shooter_id: &str, target_id: &str) -> bool {
        use crate::game::collision::shot_blocked;
        const EYE: f32 = 8.0;
        let lobbies = self.lobbies.lock().unwrap();
        if let Some(lobby) = lobbies.get(lobby_id) {
            let s = match lobby.players.get(shooter_id) { Some(p) => p, None => return false };
            let t = match lobby.players.get(target_id) { Some(p) => p, None => return false };
            let shooter = (s.x, s.y + EYE, s.z);
            let target = (t.x, t.y + EYE, t.z);
            return shot_blocked(shooter, target, &lobby.structures);
        }
        false
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

    pub fn get_lobby_scoreboard_for(&self, lobby_id: &str, viewer_team: Option<&str>) -> Vec<crate::network::messages::ScoreboardPlayer> {
        let lobbies = self.lobbies.lock().unwrap();
        if let Some(lobby) = lobbies.get(lobby_id) {
            lobby.get_scoreboard_data_for(viewer_team)
        } else {
            Vec::new()
        }
    }

    fn broadcast_scoreboard_for_lobby(lobby: &Lobby, broadcaster: &crate::network::MessageBroadcaster) {
        for player in lobby.players.values() {
            let scoreboard = lobby.get_scoreboard_data_for(player.team.as_deref());
            let msg = crate::network::messages::ServerMessage::ScoreboardUpdate { players: scoreboard };
            broadcaster.send_to_player(&player.id, &msg);
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

    pub fn plant_bomb(&self, lobby_id: &str, player_id: &str, position_x: Option<f32>, position_z: Option<f32>) -> Option<(f32, f32)> {
        let mut lobbies = self.lobbies.lock().unwrap();
        if let Some(lobby) = lobbies.get_mut(lobby_id) {
            lobby.plant_bomb(player_id, position_x, position_z)
        } else {
            None
        }
    }

    pub fn defuse_bomb(&self, lobby_id: &str, player_id: &str) -> Option<TeamColor> {
        let mut lobbies = self.lobbies.lock().unwrap();
        if let Some(lobby) = lobbies.get_mut(lobby_id) {
            lobby.defuse_bomb(player_id)
        } else {
            None
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

    pub fn take_match_end_winner(&self, lobby_id: &str) -> Option<&'static str> {
        let mut lobbies = self.lobbies.lock().unwrap();
        lobbies.get_mut(lobby_id).and_then(|l| l.take_match_end_winner())
    }

    pub fn is_match_over(&self, lobby_id: &str) -> bool {
        let lobbies = self.lobbies.lock().unwrap();
        lobbies.get(lobby_id)
            .map(|l| matches!(l.game_state, crate::game::lobby::GameState::MatchEnd))
            .unwrap_or(false)
    }

    pub fn start_new_round(&self, lobby_id: &str) -> bool {
        let mut lobbies = self.lobbies.lock().unwrap();
        if let Some(lobby) = lobbies.get_mut(lobby_id) {
            lobby.start_new_round()
        } else {
            false
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

                    let msg = crate::network::messages::ServerMessage::BuildPhaseEnd {
                        round_time: crate::game::lobby::ROUND_TIMER_SECS,
                    };
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

                    let exploded_msg = crate::network::messages::ServerMessage::BombExploded;
                    broadcaster.broadcast_to_lobby(&lobby_id, &exploded_msg, None);
                    log::info!("Bomb exploded in lobby {}", lobby_id);

                    let (orange_score, red_score) = (lobby.orange_score, lobby.red_score);
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
                    log::info!("Round ended in lobby {}, {} team wins", lobby_id, winner);

                    for player in lobby.players.values() {
                        let money_msg = crate::network::messages::ServerMessage::MoneyUpdate {
                            player_id: player.id.clone(),
                            money: player.money,
                        };
                        broadcaster.send_to_player(&player.id, &money_msg);
                        log::info!("Sent money update to player {}: ${}", player.id, player.money);
                    }
                    Self::broadcast_scoreboard_for_lobby(lobby, broadcaster);
                }
            }
        }
    }

    pub fn check_all_round_timeouts(&self, broadcaster: &crate::network::MessageBroadcaster) {
        let mut lobbies = self.lobbies.lock().unwrap();
        let lobby_ids: Vec<String> = lobbies.keys().cloned().collect();

        for lobby_id in lobby_ids {
            if let Some(lobby) = lobbies.get_mut(&lobby_id) {
                if let Some(winner_team) = lobby.check_round_timeout() {
                    let (orange_score, red_score) = (lobby.orange_score, lobby.red_score);
                    let winner = match winner_team {
                        crate::game::TeamColor::Orange => "orange",
                        crate::game::TeamColor::Red => "red",
                    };
                    let msg = crate::network::messages::ServerMessage::RoundEnd {
                        winner: winner.to_string(),
                        reason: "Time Up".to_string(),
                        orange_score,
                        red_score,
                    };
                    broadcaster.broadcast_to_lobby(&lobby_id, &msg, None);
                    log::info!("Round timed out in lobby {}, {} (defenders) win", lobby_id, winner);

                    for player in lobby.players.values() {
                        let money_msg = crate::network::messages::ServerMessage::MoneyUpdate {
                            player_id: player.id.clone(),
                            money: player.money,
                        };
                        broadcaster.send_to_player(&player.id, &money_msg);
                    }
                    Self::broadcast_scoreboard_for_lobby(lobby, broadcaster);
                }
            }
        }
    }

    pub fn check_all_match_ends(&self, broadcaster: &crate::network::MessageBroadcaster) {
        let mut lobbies = self.lobbies.lock().unwrap();
        let lobby_ids: Vec<String> = lobbies.keys().cloned().collect();

        for lobby_id in lobby_ids {
            if let Some(lobby) = lobbies.get_mut(&lobby_id) {
                if let Some(winner) = lobby.take_match_end_winner() {
                    let msg = crate::network::messages::ServerMessage::MatchEnd {
                        winner: winner.to_string(),
                    };
                    broadcaster.broadcast_to_lobby(&lobby_id, &msg, None);
                    log::info!("🏆 MATCH ENDED in lobby {}: {} wins ({}-{})",
                        lobby_id, winner, lobby.orange_score, lobby.red_score);
                }
            }
        }
    }

    pub fn check_all_round_restarts(&self, broadcaster: &crate::network::MessageBroadcaster) {
        let mut lobbies = self.lobbies.lock().unwrap();
        let lobby_ids: Vec<String> = lobbies.keys().cloned().collect();

        for lobby_id in lobby_ids {
            if let Some(lobby) = lobbies.get_mut(&lobby_id) {
                let (restarted, sides_switched) = lobby.check_round_restart();
                if restarted {

                    if sides_switched {
                        lobby.structures.clear();
                        let reset_msg = crate::network::messages::ServerMessage::MapReset;
                        broadcaster.broadcast_to_lobby(&lobby_id, &reset_msg, None);
                        log::info!("Halftime map reset broadcast for lobby {}", lobby_id);
                    }

                    for player in lobby.players.values() {
                        let respawn_msg = crate::network::messages::ServerMessage::PlayerRespawned {
                            player: player.clone()
                        };
                        broadcaster.broadcast_to_lobby(&lobby_id, &respawn_msg, None);
                    }

                    let round_msg = crate::network::messages::ServerMessage::RoundStart {
                        round_number: lobby.round_number,
                        buy_time: 30,
                        orange_score: lobby.orange_score,
                        red_score: lobby.red_score,
                        attacking_team: match lobby.attacking_team {
                            crate::game::TeamColor::Orange => "orange".to_string(),
                            crate::game::TeamColor::Red => "red".to_string(),
                        },
                    };
                    broadcaster.broadcast_to_lobby(&lobby_id, &round_msg, None);
                    log::info!("Round {} started in lobby {}", lobby.round_number, lobby_id);

                    for player in lobby.players.values() {
                        let money_msg = crate::network::messages::ServerMessage::MoneyUpdate {
                            player_id: player.id.clone(),
                            money: player.money,
                        };
                        broadcaster.send_to_player(&player.id, &money_msg);
                        log::info!("Round start: Sent money update to player {}: ${}", player.id, player.money);
                    }
                    Self::broadcast_scoreboard_for_lobby(lobby, broadcaster);

                    if let Some(bomb_carrier_id) = lobby.bomb_carrier_id.clone() {
                        let bomb_msg = crate::network::messages::ServerMessage::GiveBomb {
                            player_id: bomb_carrier_id.clone(),
                        };
                        broadcaster.broadcast_to_lobby(&lobby_id, &bomb_msg, None);
                        log::info!("Round start: gave bomb to {}", bomb_carrier_id);
                    }
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

    pub fn update_player_money(&self, lobby_id: &str, player_id: &str, new_money: i32) {
        let mut lobbies = self.lobbies.lock().unwrap();
        if let Some(lobby) = lobbies.get_mut(lobby_id) {
            if let Some(player) = lobby.players.get_mut(player_id) {
                player.money = new_money;
                log::info!("Updated player {} money to ${} in lobby {}", player_id, new_money, lobby_id);
            }
        }
    }
}