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

    pub fn handle_message(&self, player_id: &str, message: ClientMessage) {
        match message {
            ClientMessage::Join { name } => {
                self.handle_join(player_id, name);
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
        }
    }

    fn handle_join(&self, player_id: &str, name: String) {
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
            self.message_broadcaster.send_to_player(player_id, &join_msg);
        }

        self.player_manager.add_player(player.clone());
        
        self.message_broadcaster.broadcast_message(
            &ServerMessage::PlayerJoined { player },
            Some(player_id)
);

        log::info!("Player {} joined at ({}, {}, {})", player_id, spawn_pos.0, spawn_pos.1, spawn_pos.2);
    }

    fn handle_move(&self, player_id: &str, x: f32, y: f32, z: f32, rotation_x: f32, rotation_y: f32) {
        self.player_manager.update_player_position(player_id, x, y, z, rotation_x, rotation_y);

        self.message_broadcaster.broadcast_message(
            &ServerMessage::PlayerMoved {
                player_id: player_id.to_string(),
                x, y, z, rotation_x, rotation_y
            },
            Some(player_id)
);
    }

    fn handle_shoot(&self, player_id: &str, start_x: f32, start_y: f32, start_z: f32, target_x: f32, target_y: f32, target_z: f32) {
        self.message_broadcaster.broadcast_message(
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

    fn handle_hit(&self, shooter_id: &str, target_player_id: String, _killed: bool) {
        // Apply damage to target player
        if let Some((died, health)) = self.player_manager.damage_player(&target_player_id, 50) {
            if died {
                // Give kill to shooter
                self.player_manager.add_kill_to_player(shooter_id);
                
                self.message_broadcaster.broadcast_message(
                    &ServerMessage::PlayerDied { 
                        player_id: target_player_id.clone(),
                        killer_id: shooter_id.to_string()
                    },
                    None
                );
                
                // Don't auto-respawn anymore, wait for player to request respawn
            } else {
                // Player hit but not dead
                self.message_broadcaster.broadcast_message(
                    &ServerMessage::PlayerHit { 
                        player_id: target_player_id, 
                        damage: 50, 
                        health
                    },
                    None
                );
            }
        }
    }
    
    fn handle_respawn(&self, player_id: &str) {
        let player_count = self.player_manager.get_player_count();
        let spawn_pos = self.spawn_system.get_spawn_position(player_count);
        
        if let Some(player) = self.player_manager.respawn_player(player_id, spawn_pos) {
            self.message_broadcaster.broadcast_message(
                &ServerMessage::PlayerRespawned { player },
                None
            );
            
            log::info!("Player {} manually respawned at ({}, {}, {})", player_id, spawn_pos.0, spawn_pos.1, spawn_pos.2);
        }
    }
}