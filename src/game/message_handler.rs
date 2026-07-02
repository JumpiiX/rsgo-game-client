use crate::network::{ClientMessage, ServerMessage, MessageBroadcaster, TeamPlayer};
use crate::game::{Player, LobbyManager, TeamColor};
use crate::game::spawn_system::SpawnSystem;
use std::sync::Arc;
use std::collections::HashMap;

pub struct MessageHandler {
    lobby_manager: Arc<LobbyManager>,
    message_broadcaster: Arc<MessageBroadcaster>,
    spawn_system: SpawnSystem,
    player_lobbies: Arc<std::sync::Mutex<HashMap<String, String>>>,
}

impl MessageHandler {
    pub fn new(
        lobby_manager: Arc<LobbyManager>,
        message_broadcaster: Arc<MessageBroadcaster>,
    ) -> Self {
        Self {
            lobby_manager,
            message_broadcaster,
            spawn_system: SpawnSystem::new(),
            player_lobbies: Arc::new(std::sync::Mutex::new(HashMap::new())),
        }
    }

    pub fn handle_message(&self, player_id: &str, message: ClientMessage) {

        if !matches!(message, ClientMessage::Move { .. }) {
            log::debug!(">>> Received message from {}: {:?}", player_id, message);
        }
        match message {
            ClientMessage::Join { name } => {
                self.handle_join(player_id, name);
            }
            ClientMessage::CreateTeamLobby { name } => {
                self.handle_create_team_lobby(player_id, name);
            }
            ClientMessage::JoinTeam { team } => {
                self.handle_join_team(player_id, team);
            }
            ClientMessage::StartTeamGame => {
                log::info!("Received StartTeamGame message from player {}", player_id);
                self.handle_start_team_game(player_id);
            }
            ClientMessage::Move { x, y, z, rotation_x, rotation_y } => {
                self.handle_move(player_id, x, y, z, rotation_x, rotation_y);
            }
            ClientMessage::Shoot { start_x, start_y, start_z, target_x, target_y, target_z } => {
                self.handle_shoot(player_id, start_x, start_y, start_z, target_x, target_y, target_z);
            }
            ClientMessage::Hit { target_player_id, killed } => {
                self.handle_hit(player_id, target_player_id, killed);
            }
            ClientMessage::Respawn => {
                self.handle_respawn(player_id);
            }
            ClientMessage::PlantBomb { position_x, position_z } => {
                log::error!("!!!!! PLANT BOMB MESSAGE RECEIVED from {} at ({:?}, {:?})", player_id, position_x, position_z);
                self.handle_plant_bomb(player_id, position_x, position_z);
            }
            ClientMessage::DefuseBomb => {
                self.handle_defuse_bomb(player_id);
            }
            ClientMessage::DropBomb => {
                self.handle_drop_bomb(player_id);
            }
            ClientMessage::PickupBomb => {
                self.handle_pickup_bomb(player_id);
            }
            ClientMessage::PlaceStructure { structure_type, x, y, z } => {
                self.handle_place_structure(player_id, structure_type, x, y, z);
            }
            ClientMessage::WallHit { wall_x, wall_z, local_x, world_y, radius } => {
                self.handle_wall_hit(player_id, wall_x, wall_z, local_x, world_y, radius);
            }
        }
    }

    fn handle_join(&self, player_id: &str, name: String) {
        let deathmatch_lobby_id = self.lobby_manager.get_deathmatch_lobby_id();
        let spawn_pos = (0.0, 5.0, 0.0);

        let player = Player::new(player_id.to_string(), name, spawn_pos);

        let existing_players = self.lobby_manager.get_lobby_players(&deathmatch_lobby_id);
        log::info!("Deathmatch lobby {} has {} existing players", deathmatch_lobby_id, existing_players.len());
        for existing_player in existing_players {
            log::info!("Sending existing player {} to new player {}", existing_player.id, player_id);
            let join_msg = ServerMessage::PlayerJoined {
                player: existing_player
            };
            self.message_broadcaster.send_to_player(player_id, &join_msg);
        }

        self.lobby_manager.join_lobby(&deathmatch_lobby_id, player.clone());
        self.player_lobbies.lock().unwrap().insert(player_id.to_string(), deathmatch_lobby_id.clone());

        self.message_broadcaster.add_player_to_lobby(&deathmatch_lobby_id, player_id);

        let welcome_msg = ServerMessage::Welcome {
            player_id: player_id.to_string(),
            lobby_id: deathmatch_lobby_id.clone(),
            game_mode: "deathmatch".to_string(),
        };
        self.message_broadcaster.send_to_player(player_id, &welcome_msg);

        self.message_broadcaster.broadcast_to_lobby(
            &deathmatch_lobby_id,
            &ServerMessage::PlayerJoined { player },
            Some(player_id)
        );

        self.broadcast_scoreboard(&deathmatch_lobby_id);

        log::info!("Player {} joined deathmatch at ({}, {}, {})", player_id, spawn_pos.0, spawn_pos.1, spawn_pos.2);
    }

    fn handle_move(&self, player_id: &str, x: f32, y: f32, z: f32, rotation_x: f32, rotation_y: f32) {
        if let Some(lobby_id) = self.player_lobbies.lock().unwrap().get(player_id).cloned() {
            self.lobby_manager.update_player_position(&lobby_id, player_id, x, y, z, rotation_x, rotation_y);

            self.message_broadcaster.broadcast_to_lobby(
                &lobby_id,
                &ServerMessage::PlayerMoved {
                    player_id: player_id.to_string(),
                    x, y, z, rotation_x, rotation_y
                },
                Some(player_id)
            );
        }
    }

    fn handle_create_team_lobby(&self, player_id: &str, name: String) {

        if let Some(old_lobby_id) = self.player_lobbies.lock().unwrap().get(player_id).cloned() {
            log::info!("Removing player {} from lobby {} to join team selection", player_id, old_lobby_id);
            self.lobby_manager.leave_lobby(&old_lobby_id, player_id);
            self.message_broadcaster.remove_player_from_lobby(&old_lobby_id, player_id);

            let msg = ServerMessage::PlayerLeft {
                player_id: player_id.to_string()
            };
            self.message_broadcaster.broadcast_to_lobby(&old_lobby_id, &msg, None);
            self.broadcast_scoreboard(&old_lobby_id);
        }

        let team_lobby_id = self.lobby_manager.get_or_create_team_selection_lobby();

        let spawn_pos = (0.0, 5.0, 0.0);
        let player = Player::new(player_id.to_string(), name, spawn_pos);
        self.lobby_manager.join_lobby(&team_lobby_id, player);

        self.player_lobbies.lock().unwrap().insert(player_id.to_string(), team_lobby_id.clone());

        self.message_broadcaster.add_player_to_lobby(&team_lobby_id, player_id);

        let msg = ServerMessage::TeamLobbyCreated {
            lobby_id: team_lobby_id.clone()
        };
        self.message_broadcaster.send_to_player(player_id, &msg);

        self.broadcast_team_update(&team_lobby_id);

        log::info!("Player {} joined team selection lobby {}", player_id, team_lobby_id);
    }

    fn handle_join_team(&self, player_id: &str, team: String) {
        if let Some(lobby_id) = self.player_lobbies.lock().unwrap().get(player_id).cloned() {
            let team_color = match team.as_str() {
                "orange" => TeamColor::Orange,
                "red" => TeamColor::Red,
                _ => return,
            };

            if self.lobby_manager.join_team(&lobby_id, player_id, team_color) {
                self.broadcast_team_update(&lobby_id);
                log::info!("Player {} joined {} team in lobby {}", player_id, team, lobby_id);
            } else {
                log::warn!("Failed to join team {} for player {} in lobby {}", team, player_id, lobby_id);
            }
        } else {
            log::warn!("Player {} tried to join team but has no lobby association", player_id);
        }
    }

    fn handle_start_team_game(&self, player_id: &str) {
        log::info!("handle_start_team_game called for player {}", player_id);
        if let Some(lobby_id) = self.player_lobbies.lock().unwrap().get(player_id).cloned() {
            log::info!("Player {} found in lobby {}", player_id, lobby_id);
            if self.lobby_manager.start_team_game(&lobby_id) {

                let lobby_players = self.lobby_manager.get_lobby_players(&lobby_id);

                for player in &lobby_players {
                    log::info!("Sending other players to {}", player.id);
                    for other_player in &lobby_players {
                        if player.id != other_player.id {
                            log::info!("Sending player {} to player {}", other_player.id, player.id);
                            let join_msg = ServerMessage::PlayerJoined {
                                player: other_player.clone()
                            };
                            self.message_broadcaster.send_to_player(&player.id, &join_msg);
                        }
                    }
                }

                let msg = ServerMessage::GameStarted {
                    game_mode: "team".to_string()
                };
                self.message_broadcaster.broadcast_to_lobby(&lobby_id, &msg, None);

                let attacking_team = self.lobby_manager.get_attacking_team(&lobby_id)
                    .map(|t| match t { crate::game::TeamColor::Red => "red", crate::game::TeamColor::Orange => "orange" })
                    .unwrap_or("red");
                let round_msg = ServerMessage::RoundStart {
                    round_number: 1,
                    buy_time: 30,
                    orange_score: 0,
                    red_score: 0,
                    attacking_team: attacking_team.to_string(),
                };
                self.message_broadcaster.broadcast_to_lobby(&lobby_id, &round_msg, None);

                for player in &lobby_players {
                    let respawn_msg = ServerMessage::PlayerRespawned {
                        player: player.clone()
                    };
                    self.message_broadcaster.broadcast_to_lobby(&lobby_id, &respawn_msg, None);
                    log::info!("Sent respawn for player {} at position ({}, {}, {})",
                        player.id, player.x, player.y, player.z);
                }

                if let Some(bomb_carrier_id) = self.lobby_manager.get_bomb_carrier(&lobby_id) {
                    let bomb_msg = ServerMessage::GiveBomb {
                        player_id: bomb_carrier_id.clone(),
                    };
                    self.message_broadcaster.send_to_player(&bomb_carrier_id, &bomb_msg);
                    log::info!("Bomb given to player {}", bomb_carrier_id);
                }

                self.broadcast_scoreboard(&lobby_id);

                log::info!("Team game started in lobby {} with {} players", lobby_id, lobby_players.len());
            }
        }
    }

    fn broadcast_team_update(&self, lobby_id: &str) {
        if let Some((orange_team, red_team)) = self.lobby_manager.get_team_data(lobby_id) {
            let orange_players: Vec<TeamPlayer> = orange_team.into_iter()
                .map(|(id, name)| TeamPlayer { id, name })
                .collect();
            let red_players: Vec<TeamPlayer> = red_team.into_iter()
                .map(|(id, name)| TeamPlayer { id, name })
                .collect();

            let can_start = orange_players.len() >= 1 && red_players.len() >= 1;

            let msg = ServerMessage::TeamUpdate {
                orange_team: orange_players,
                red_team: red_players,
                can_start,
            };

            self.message_broadcaster.broadcast_to_lobby(lobby_id, &msg, None);
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn handle_shoot(&self, player_id: &str, start_x: f32, start_y: f32, start_z: f32, target_x: f32, target_y: f32, target_z: f32) {
        if let Some(lobby_id) = self.player_lobbies.lock().unwrap().get(player_id).cloned() {
            self.message_broadcaster.broadcast_to_lobby(
                &lobby_id,
                &ServerMessage::PlayerShot {
                    shooter_id: player_id.to_string(),
                    start_x,
                    start_y,
                    start_z,
                    target_x,
                    target_y,
                    target_z,
                },
                Some(player_id)
            );
        }
    }

    fn handle_hit(&self, shooter_id: &str, target_player_id: String, _killed: bool) {
        if let Some(lobby_id) = self.player_lobbies.lock().unwrap().get(shooter_id).cloned() {

            if self.lobby_manager.shot_is_blocked(&lobby_id, shooter_id, &target_player_id) {
                log::info!("Hit rejected: wall between {} and {}", shooter_id, target_player_id);
                return;
            }
            if let Some((died, health, shield)) = self.lobby_manager.damage_player(&lobby_id, &target_player_id, 50) {
                if died {
                    self.lobby_manager.add_kill_to_player(&lobby_id, shooter_id);

                    let lobby_players = self.lobby_manager.get_lobby_players(&lobby_id);
                    let victim_name = lobby_players.iter().find(|p| p.id == target_player_id)
                        .map(|p| p.name.clone())
                        .unwrap_or_else(|| "Unknown".to_string());
                    let killer_name = lobby_players.iter().find(|p| p.id == shooter_id)
                        .map(|p| p.name.clone())
                        .unwrap_or_else(|| "Unknown".to_string());

                    if let Some(killer) = lobby_players.iter().find(|p| p.id == shooter_id) {
                        self.message_broadcaster.send_to_player(
                            shooter_id,
                            &ServerMessage::MoneyUpdate {
                                player_id: shooter_id.to_string(),
                                money: killer.money
                            }
                        );
                    }

                    self.message_broadcaster.broadcast_to_lobby(
                        &lobby_id,
                        &ServerMessage::PlayerDied {
                            player_id: target_player_id.clone(),
                            killer_id: shooter_id.to_string(),
                            victim_name,
                            killer_name
                        },
                        None
                    );

                    if let Some(position) = self.lobby_manager.drop_bomb(&lobby_id, &target_player_id) {
                        let msg = ServerMessage::BombDropped {
                            position_x: position.0,
                            position_y: position.1,
                            position_z: position.2,
                        };
                        self.message_broadcaster.broadcast_to_lobby(&lobby_id, &msg, None);
                        log::info!("Dead player {} dropped bomb at ({}, {}, {})", target_player_id, position.0, position.1, position.2);
                    }

                    self.broadcast_scoreboard(&lobby_id);

                    if let Some(winning_team) = self.lobby_manager.check_team_elimination(&lobby_id) {
                        self.handle_round_end(&lobby_id, winning_team, "Team Eliminated");
                    }

                } else {
                    self.message_broadcaster.broadcast_to_lobby(
                        &lobby_id,
                        &ServerMessage::PlayerHit {
                            player_id: target_player_id,
                            damage: 50,
                            health,
                            shield
                        },
                        None
                    );
                }
            } else {
                log::debug!("Hit rejected on player {} by {}", target_player_id, shooter_id);
            }
        }
    }

    fn handle_respawn(&self, player_id: &str) {
        if let Some(lobby_id) = self.player_lobbies.lock().unwrap().get(player_id).cloned() {

            if !self.lobby_manager.can_respawn(&lobby_id) {
                log::info!("Player {} tried to respawn but it's not allowed during round", player_id);
                return;
            }

            if let Some(player) = self.lobby_manager.respawn_player(&lobby_id, player_id) {
                self.message_broadcaster.broadcast_to_lobby(
                    &lobby_id,
                    &ServerMessage::PlayerRespawned { player: player.clone() },
                    None
                );

                log::info!("Player {} respawned at ({}, {}, {})", player_id, player.x, player.y, player.z);
            }
        }
    }

    fn broadcast_scoreboard(&self, lobby_id: &str) {
        log::info!("Broadcasting scoreboard update for lobby {}", lobby_id);
        let scoreboard_data = self.lobby_manager.get_lobby_scoreboard(lobby_id);
        log::info!("Scoreboard data: {:?}", scoreboard_data);
        let scoreboard_message = ServerMessage::ScoreboardUpdate {
            players: scoreboard_data,
        };

        self.message_broadcaster.broadcast_to_lobby(lobby_id, &scoreboard_message, None);
        log::info!("Scoreboard broadcast complete for lobby {}", lobby_id);
    }

    pub fn handle_player_disconnect(&self, player_id: &str) {
        if let Some(lobby_id) = self.player_lobbies.lock().unwrap().remove(player_id) {

            self.message_broadcaster.remove_player_from_lobby(&lobby_id, player_id);

            self.lobby_manager.leave_lobby(&lobby_id, player_id);

            let msg = ServerMessage::PlayerLeft {
                player_id: player_id.to_string()
            };
            self.message_broadcaster.broadcast_to_lobby(&lobby_id, &msg, None);

            self.broadcast_team_update(&lobby_id);
            self.broadcast_scoreboard(&lobby_id);

            log::info!("Player {} disconnected from lobby {}", player_id, lobby_id);
        }
    }

    fn handle_plant_bomb(&self, player_id: &str, position_x: Option<f32>, position_z: Option<f32>) {
        log::error!("=== PLANT BOMB REQUEST from player {} at position ({:?}, {:?})", player_id, position_x, position_z);

        if let Some(lobby_id) = self.player_lobbies.lock().unwrap().get(player_id).cloned() {
            log::error!("Player {} is in lobby {}", player_id, lobby_id);

            if let Some((x, z)) = self.lobby_manager.plant_bomb(&lobby_id, player_id, position_x, position_z) {
                let msg = ServerMessage::BombPlanted {
                    player_id: player_id.to_string(),
                    timer: crate::game::lobby::BOMB_TIMER_SECS as i32,
                    position_x: x,
                    position_z: z,
                };
                self.message_broadcaster.broadcast_to_lobby(&lobby_id, &msg, None);
                log::error!("✅ BOMB PLANTED SUCCESS: Player {} planted the bomb at ({}, {})", player_id, x, z);
            } else {
                log::error!("❌ BOMB PLANT FAILED for player {} - check game state and bomb carrier", player_id);
            }
        } else {
            log::error!("❌ Player {} not found in any lobby!", player_id);
        }
    }

    fn handle_defuse_bomb(&self, player_id: &str) {
        if let Some(lobby_id) = self.player_lobbies.lock().unwrap().get(player_id).cloned() {

            if let Some(winning_team) = self.lobby_manager.defuse_bomb(&lobby_id, player_id) {

                let defused_msg = ServerMessage::BombDefused {
                    player_id: player_id.to_string(),
                };
                self.message_broadcaster.broadcast_to_lobby(&lobby_id, &defused_msg, None);
                log::info!("Player {} defused the bomb in lobby {}", player_id, lobby_id);

                self.finish_round(&lobby_id, winning_team, "Bomb Defused", true);
            }
        }
    }

    fn handle_place_structure(&self, player_id: &str, structure_type: String, x: f32, y: f32, z: f32) {

        if let Some(lobby_id) = self.player_lobbies.lock().unwrap().get(player_id).cloned() {

            let cost = match structure_type.as_str() {
                "large" => 800,
                "destructible" => 600,
                "small" | "barrier" => 400,
                _ => 0,
            };

            if cost > 0 {
                if let Some(mut player) = self.lobby_manager.get_player(&lobby_id, player_id) {
                    if player.money >= cost {
                        player.money -= cost;

                        self.lobby_manager.update_player_money(&lobby_id, player_id, player.money);

                        log::info!("Player {} placed {} at ({}, rotation: {}, {}) for ${}, money: ${}",
                            player_id, structure_type, x, y, z, cost, player.money);

                        let money_msg = ServerMessage::MoneyUpdate {
                            player_id: player_id.to_string(),
                            money: player.money,
                        };
                        self.message_broadcaster.send_to_player(player_id, &money_msg);
                    } else {
                        log::warn!("Player {} tried to place {} but only has ${} (needs ${})",
                            player_id, structure_type, player.money, cost);
                        return;
                    }
                }
            }

            self.lobby_manager.add_structure(&lobby_id, &structure_type, x, z, y);

            let msg = ServerMessage::StructurePlaced {
                player_id: player_id.to_string(),
                structure_type,
                x,
                y,
                z,
            };

            self.message_broadcaster.broadcast_to_lobby(&lobby_id, &msg, Some(player_id));
        }
    }

    fn handle_wall_hit(&self, player_id: &str, wall_x: f32, wall_z: f32, local_x: f32, world_y: f32, radius: f32) {
        if let Some(lobby_id) = self.player_lobbies.lock().unwrap().get(player_id).cloned() {

            if self.lobby_manager.add_wall_hole(&lobby_id, wall_x, wall_z, local_x, world_y, radius) {
                self.message_broadcaster.broadcast_to_lobby(
                    &lobby_id,
                    &ServerMessage::WallHole { wall_x, wall_z, local_x, world_y, radius },
                    None,
                );
            }
        }
    }

    fn handle_round_end(&self, lobby_id: &str, winning_team: crate::game::TeamColor, reason: &str) {
        self.finish_round(lobby_id, winning_team, reason, false);
    }

    fn finish_round(&self, lobby_id: &str, winning_team: crate::game::TeamColor, reason: &str, already_ended: bool) {

        if !already_ended {
            self.lobby_manager.end_round(lobby_id, winning_team.clone(), reason);
        }

        let (orange_score, red_score) = self.lobby_manager.get_team_scores(lobby_id);

        let winner_str = match winning_team {
            crate::game::TeamColor::Orange => "orange",
            crate::game::TeamColor::Red => "red",
        };

        let msg = ServerMessage::RoundEnd {
            winner: winner_str.to_string(),
            reason: reason.to_string(),
            orange_score,
            red_score,
        };
        self.message_broadcaster.broadcast_to_lobby(lobby_id, &msg, None);

        let players = self.lobby_manager.get_lobby_players(lobby_id);
        for player in players {
            let money_msg = ServerMessage::MoneyUpdate {
                player_id: player.id.clone(),
                money: player.money,
            };
            self.message_broadcaster.send_to_player(&player.id, &money_msg);
            log::info!("Sent money update after round end to player {}: ${}", player.id, player.money);
        }

        if self.lobby_manager.is_match_over(lobby_id) {
            if let Some(winner) = self.lobby_manager.take_match_end_winner(lobby_id) {
                self.message_broadcaster.broadcast_to_lobby(
                    lobby_id,
                    &ServerMessage::MatchEnd { winner: winner.to_string() },
                    None,
                );
                log::info!("🏆 MATCH ENDED in lobby {}: {} wins", lobby_id, winner);
            }
            return;
        }

        let lobby_id = lobby_id.to_string();
        let broadcaster = self.message_broadcaster.clone();
        let lobby_manager = self.lobby_manager.clone();

        std::thread::spawn(move || {

            std::thread::sleep(std::time::Duration::from_secs(5));

            if lobby_manager.is_match_over(&lobby_id) {
                if let Some(winner) = lobby_manager.take_match_end_winner(&lobby_id) {
                    broadcaster.broadcast_to_lobby(
                        &lobby_id,
                        &ServerMessage::MatchEnd { winner: winner.to_string() },
                        None,
                    );
                    log::info!("🏆 MATCH ENDED in lobby {}: {} wins", lobby_id, winner);
                }
                return;
            }

            let sides_switched = lobby_manager.start_new_round(&lobby_id);

            if sides_switched {
                lobby_manager.clear_structures(&lobby_id);
                broadcaster.broadcast_to_lobby(&lobby_id, &ServerMessage::MapReset, None);
                log::info!("Halftime map reset broadcast for lobby {}", lobby_id);
            }

            let (orange_score, red_score) = lobby_manager.get_team_scores(&lobby_id);

            let attacking_team = lobby_manager.get_attacking_team(&lobby_id)
                .map(|t| match t { crate::game::TeamColor::Red => "red", crate::game::TeamColor::Orange => "orange" })
                .unwrap_or("red");
            let round_msg = ServerMessage::RoundStart {
                round_number: lobby_manager.get_round_number(&lobby_id),
                buy_time: 30,
                orange_score,
                red_score,
                attacking_team: attacking_team.to_string(),
            };
            broadcaster.broadcast_to_lobby(&lobby_id, &round_msg, None);

            let players = lobby_manager.get_lobby_players(&lobby_id);
            for player in players.iter() {
                let respawn_msg = ServerMessage::PlayerRespawned {
                    player: player.clone()
                };
                broadcaster.broadcast_to_lobby(&lobby_id, &respawn_msg, None);
            }

            for player in players {
                let money_msg = ServerMessage::MoneyUpdate {
                    player_id: player.id.clone(),
                    money: player.money,
                };
                broadcaster.send_to_player(&player.id, &money_msg);
                log::info!("Sent money update at round start to player {}: ${}", player.id, player.money);
            }

            if let Some(bomb_carrier_id) = lobby_manager.get_bomb_carrier(&lobby_id) {
                let bomb_msg = ServerMessage::GiveBomb {
                    player_id: bomb_carrier_id.clone(),
                };
                broadcaster.send_to_player(&bomb_carrier_id, &bomb_msg);
            }
        });
    }

    fn handle_drop_bomb(&self, player_id: &str) {
        if let Some(lobby_id) = self.player_lobbies.lock().unwrap().get(player_id).cloned() {
            if let Some(position) = self.lobby_manager.drop_bomb(&lobby_id, player_id) {
                let msg = ServerMessage::BombDropped {
                    position_x: position.0,
                    position_y: position.1,
                    position_z: position.2,
                };
                self.message_broadcaster.broadcast_to_lobby(&lobby_id, &msg, None);
                log::info!("Player {} dropped bomb at ({}, {}, {})", player_id, position.0, position.1, position.2);
            }
        }
    }

    fn handle_pickup_bomb(&self, player_id: &str) {
        if let Some(lobby_id) = self.player_lobbies.lock().unwrap().get(player_id).cloned() {
            if let Some(player_name) = self.lobby_manager.pickup_bomb(&lobby_id, player_id) {
                let msg = ServerMessage::BombPickedUp {
                    player_id: player_id.to_string(),
                    player_name,
                };
                self.message_broadcaster.broadcast_to_lobby(&lobby_id, &msg, None);
                log::info!("Player {} picked up bomb", player_id);
            }
        }
    }
}