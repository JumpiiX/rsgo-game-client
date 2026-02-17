use crate::network::{ClientMessage, ServerMessage, MessageBroadcaster};
use crate::game::{Player, PlayerManager};
use crate::game::spawn_system::SpawnSystem;
use std::sync::Arc;

pub struct MessageHandler {
    player_manager: Arc<PlayerManager>,
    message_broadcaster: Arc<MessageBroadcaster>,
    spawn_system: SpawnSystem,
}

impl MessageHandler {
    pub fn new(
        player_manager: Arc<PlayerManager>,
        message_broadcaster: Arc<MessageBroadcaster>,
    ) -> Self {
        Self {
            player_manager,
            message_broadcaster,
            spawn_system: SpawnSystem::new(),
        }
    }

    pub async fn handle_message(&self, player_id: &str, message: ClientMessage) {
        match message {
            ClientMessage::Join { name } => {
                self.handle_join(player_id, name).await;
            }
            ClientMessage::Move { x, y, z, rotation_x, rotation_y } => {
                self.handle_move(player_id, x, y, z, rotation_x, rotation_y).await;
            }
            ClientMessage::Shoot { target_x, target_y, target_z } => {
                self.handle_shoot(player_id, target_x, target_y, target_z).await;
            }
            ClientMessage::Hit { target_player_id, killed } => {
                self.handle_hit(player_id, target_player_id, killed).await;
            }
        }
    }

    async fn handle_join(&self, player_id: &str, name: String) {
        // Get spawn position based on current player count
        let player_count = self.player_manager.get_player_count();
        let spawn_pos = self.spawn_system.get_spawn_position(player_count);

        let player = Player::new(player_id.to_string(), name, spawn_pos);

        // Send existing players to new player first
        let existing_players = self.player_manager.get_all_players();
        for existing_player in existing_players {
            let join_msg = ServerMessage::PlayerJoined { 
                player: existing_player 
            };
            self.message_broadcaster.send_to_player(player_id, join_msg);
        }

        self.player_manager.add_player(player.clone());
        
        self.message_broadcaster.broadcast_message(
            ServerMessage::PlayerJoined { player },
            Some(player_id)
        ).await;

        log::info!("Player {} joined at ({}, {}, {})", player_id, spawn_pos.0, spawn_pos.1, spawn_pos.2);
    }

    async fn handle_move(&self, player_id: &str, x: f32, y: f32, z: f32, rotation_x: f32, rotation_y: f32) {
        self.player_manager.update_player_position(player_id, x, y, z, rotation_x, rotation_y);

        self.message_broadcaster.broadcast_message(
            ServerMessage::PlayerMoved {
                player_id: player_id.to_string(),
                x, y, z, rotation_x, rotation_y
            },
            Some(player_id)
        ).await;
    }

    async fn handle_shoot(&self, player_id: &str, target_x: f32, target_y: f32, target_z: f32) {
        if let Some(shooter) = self.player_manager.get_player(player_id) {
            self.message_broadcaster.broadcast_message(
                ServerMessage::PlayerShot {
                    shooter_id: player_id.to_string(),
                    start_x: shooter.x,
                    start_y: shooter.y,
                    start_z: shooter.z,
                    target_x,
                    target_y,
                    target_z,
                },
                Some(player_id)
            ).await;
        }
    }

    async fn handle_hit(&self, shooter_id: &str, target_player_id: String, killed: bool) {
        if killed {
            self.message_broadcaster.broadcast_message(
                ServerMessage::PlayerDied { 
                    player_id: target_player_id.clone() 
                },
                None
            ).await;

            tokio::spawn({
                let player_manager = Arc::clone(&self.player_manager);
                let message_broadcaster = Arc::clone(&self.message_broadcaster);
                let spawn_system = SpawnSystem::new();
                async move {
                    tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
                    
                    if let Some(mut player) = player_manager.get_player(&target_player_id) {
                        let player_count = player_manager.get_player_count();
                        let spawn_pos = spawn_system.get_spawn_position(player_count);
                        player.x = spawn_pos.0;
                        player.y = spawn_pos.1;
                        player.z = spawn_pos.2;
                        
                        player_manager.update_player_position(&target_player_id, spawn_pos.0, spawn_pos.1, spawn_pos.2, 0.0, 0.0);
                        
                        message_broadcaster.broadcast_message(
                            ServerMessage::PlayerRespawned { player },
                            None
                        ).await;
                    }
                }
            });
        } else {
            self.message_broadcaster.broadcast_message(
                ServerMessage::PlayerHit { 
                    player_id: target_player_id, 
                    damage: 1, 
                    health: 4
                },
                None
            ).await;
        }
    }
}