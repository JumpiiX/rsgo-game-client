use crate::network::{WebSocketHandler, MessageBroadcaster};
use crate::game::{LobbyManager, MessageHandler};
use std::net::SocketAddr;
use tokio::net::TcpStream;
use std::sync::Arc;
use uuid::Uuid;

pub struct GameServer {
    lobby_manager: Arc<LobbyManager>,
    message_broadcaster: Arc<MessageBroadcaster>,
    message_handler: Arc<MessageHandler>,
}

impl GameServer {
    pub fn new() -> Self {
        let lobby_manager = Arc::new(LobbyManager::new());
        let message_broadcaster = Arc::new(MessageBroadcaster::new());
        let message_handler = Arc::new(MessageHandler::new(
            Arc::clone(&lobby_manager),
            Arc::clone(&message_broadcaster),
        ));

        // Note: Shield updates and scoreboards are now handled per-lobby in the message handler
        // No need for global periodic updates that broadcast to all players
        
        // Start a timer to check build phase timeouts and bomb explosions
        let lobby_manager_clone = Arc::clone(&lobby_manager);
        let message_broadcaster_clone = Arc::clone(&message_broadcaster);
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(1));
            loop {
                interval.tick().await;
                // Check all lobbies for build phase timeout
                lobby_manager_clone.check_all_build_phase_timeouts(&message_broadcaster_clone);
                // Check all lobbies for bomb explosions
                lobby_manager_clone.check_all_bomb_explosions(&message_broadcaster_clone);
            }
        });

        Self {
            lobby_manager,
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
        ).await;

        if let Err(e) = ws_handler {
            log::error!("Failed to create WebSocket handler: {e}");
            return;
        }

        ws_handler.unwrap().handle().await;
    }
}