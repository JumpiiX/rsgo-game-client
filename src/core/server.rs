use crate::network::{WebSocketHandler, MessageBroadcaster};
use crate::game::{PlayerManager, MessageHandler};
use std::net::SocketAddr;
use tokio::net::TcpStream;
use std::sync::Arc;
use uuid::Uuid;
use tokio::time::{Duration, interval};

pub struct GameServer {
    player_manager: Arc<PlayerManager>,
    message_broadcaster: Arc<MessageBroadcaster>,
    message_handler: Arc<MessageHandler>,
}

impl GameServer {
    pub fn new() -> Self {
        let player_manager = Arc::new(PlayerManager::new());
        let message_broadcaster = Arc::new(MessageBroadcaster::new());
        let message_handler = Arc::new(MessageHandler::new(
            Arc::clone(&player_manager),
            Arc::clone(&message_broadcaster),
        ));

        let player_manager_clone = Arc::clone(&player_manager);
        let message_broadcaster_clone = Arc::clone(&message_broadcaster);
        tokio::spawn(async move {
            let mut interval = interval(Duration::from_millis(100));
            loop {
                interval.tick().await;
                let shield_updates = player_manager_clone.update_shields();
                
                for (player_id, shield) in shield_updates {
                    message_broadcaster_clone.send_to_player(
                        &player_id, 
                        &crate::network::messages::ServerMessage::ShieldUpdate { 
                            player_id: player_id.clone(), 
                            shield 
                        }
                    );
                }
            }
        });

        let player_manager_scoreboard = Arc::clone(&player_manager);
        let message_broadcaster_scoreboard = Arc::clone(&message_broadcaster);
        tokio::spawn(async move {
            let mut interval = interval(Duration::from_secs(5));
            loop {
                interval.tick().await;
                let scoreboard_data = player_manager_scoreboard.get_scoreboard_data();
                if !scoreboard_data.is_empty() {
                    let scoreboard_message = crate::network::messages::ServerMessage::ScoreboardUpdate {
                        players: scoreboard_data,
                    };
                    message_broadcaster_scoreboard.broadcast_message(&scoreboard_message, None);
                }
            }
        });

        Self {
            player_manager,
            message_broadcaster,
            message_handler,
        }
    }

    pub async fn handle_connection(&self, stream: TcpStream, addr: SocketAddr) {
        let player_id = Uuid::new_v4().to_string();
        log::info!("New connection from {addr}, assigned ID: {player_id}");

        let ws_handler = WebSocketHandler::new(
            stream,
            player_id.clone(),
            Arc::clone(&self.message_broadcaster),
            Arc::clone(&self.message_handler),
            Arc::clone(&self.player_manager),
        ).await;

        if let Err(e) = ws_handler {
            log::error!("Failed to create WebSocket handler: {e}");
            return;
        }

        ws_handler.unwrap().handle().await;
    }
}