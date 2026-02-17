use crate::network::{MessageBroadcaster, ClientMessage, ServerMessage};
use crate::game::{MessageHandler, PlayerManager};
use tokio::net::TcpStream;
use tokio_tungstenite::{accept_async, tungstenite::Message, WebSocketStream};
use futures_util::{SinkExt, StreamExt};
use std::sync::Arc;

pub struct WebSocketHandler {
    player_id: String,
    ws_receiver: futures_util::stream::SplitStream<WebSocketStream<TcpStream>>,
    message_broadcaster: Arc<MessageBroadcaster>,
    message_handler: Arc<MessageHandler>,
    player_manager: Arc<PlayerManager>,
}

impl WebSocketHandler {
    pub async fn new(
        stream: TcpStream,
        player_id: String,
        message_broadcaster: Arc<MessageBroadcaster>,
        message_handler: Arc<MessageHandler>,
        player_manager: Arc<PlayerManager>,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let ws_stream = accept_async(stream).await?;
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<String>();
        
        message_broadcaster.add_connection(player_id.clone(), tx);

        let welcome_msg = ServerMessage::Welcome {
            player_id: player_id.clone(),
        };
        
        let (mut ws_sender, ws_receiver) = ws_stream.split();
        
        let welcome_json = serde_json::to_string(&welcome_msg).unwrap();
        ws_sender.send(Message::Text(welcome_json)).await?;
        
        // Spawn task to handle outgoing messages
        tokio::spawn(async move {
            while let Some(msg) = rx.recv().await {
                if ws_sender.send(Message::Text(msg)).await.is_err() {
                    break;
                }
            }
        });

        Ok(Self {
            player_id,
            ws_receiver,
            message_broadcaster,
            message_handler,
            player_manager,
        })
    }

    pub async fn handle(mut self) {
        while let Some(msg) = self.ws_receiver.next().await {
            match msg {
                Ok(Message::Text(text)) => {
                    if let Ok(client_msg) = serde_json::from_str::<ClientMessage>(&text) {
                        self.message_handler.handle_message(&self.player_id, client_msg).await;
                    }
                }
                Ok(Message::Close(_)) | Err(_) => break,
                _ => {}
            }
        }

        // Cleanup on disconnect
        self.message_broadcaster.remove_connection(&self.player_id);
        self.player_manager.remove_player(&self.player_id);
        
        self.message_broadcaster.broadcast_message(
            ServerMessage::PlayerLeft { player_id: self.player_id.clone() },
            None
        ).await;

        log::info!("Player {} disconnected", self.player_id);
    }
}