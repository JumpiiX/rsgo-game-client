use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use tokio::net::{TcpListener, TcpStream};
use tokio_tungstenite::{accept_async, tungstenite::Message};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Player {
    id: String,
    name: String,
    x: f32,
    y: f32,
    z: f32,
    rotation_x: f32,
    rotation_y: f32,
    health: i32,
    alive: bool,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
enum ClientMessage {
    #[serde(rename = "join")]
    Join { name: String },
    #[serde(rename = "move")]
    Move { x: f32, y: f32, z: f32, rotation_x: f32, rotation_y: f32 },
    #[serde(rename = "shoot")]
    Shoot { target_x: f32, target_y: f32, target_z: f32 },
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
enum ServerMessage {
    #[serde(rename = "welcome")]
    Welcome { player_id: String },
    #[serde(rename = "player_joined")]
    PlayerJoined { player: Player },
    #[serde(rename = "player_left")]
    PlayerLeft { player_id: String },
    #[serde(rename = "player_moved")]
    PlayerMoved { player_id: String, x: f32, y: f32, z: f32, rotation_x: f32, rotation_y: f32 },
    #[serde(rename = "player_shot")]
    PlayerShot { shooter_id: String, target_x: f32, target_y: f32, target_z: f32 },
    #[serde(rename = "player_hit")]
    PlayerHit { player_id: String, damage: i32, health: i32 },
    #[serde(rename = "player_died")]
    PlayerDied { player_id: String },
    #[serde(rename = "player_respawned")]
    PlayerRespawned { player: Player },
}

type Players = Arc<Mutex<HashMap<String, Player>>>;
type Connections = Arc<Mutex<HashMap<String, tokio::sync::mpsc::UnboundedSender<String>>>>;

struct GameServer {
    players: Players,
    connections: Connections,
}

impl GameServer {
    fn new() -> Self {
        Self {
            players: Arc::new(Mutex::new(HashMap::new())),
            connections: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    async fn broadcast_message(&self, message: ServerMessage, exclude_id: Option<&str>) {
        let msg_json = serde_json::to_string(&message).unwrap();
        let connections = self.connections.lock().unwrap().clone();
        
        for (id, sender) in connections.iter() {
            if let Some(exclude) = exclude_id {
                if id == exclude {
                    continue;
                }
            }
            
            let _ = sender.send(msg_json.clone());
        }
    }

    async fn handle_client_message(&self, player_id: &str, message: ClientMessage) {
        match message {
            ClientMessage::Join { name } => {
                let player = Player {
                    id: player_id.to_string(),
                    name,
                    x: 0.0,
                    y: 10.0,
                    z: 0.0,
                    rotation_x: 0.0,
                    rotation_y: 0.0,
                    health: 100,
                    alive: true,
                };

                self.players.lock().unwrap().insert(player_id.to_string(), player.clone());
                
                self.broadcast_message(
                    ServerMessage::PlayerJoined { player },
                    Some(player_id)
                ).await;

                log::info!("Player {} joined the game", player_id);
            }
            ClientMessage::Move { x, y, z, rotation_x, rotation_y } => {
                {
                    let mut players = self.players.lock().unwrap();
                    if let Some(player) = players.get_mut(player_id) {
                        player.x = x;
                        player.y = y;
                        player.z = z;
                        player.rotation_x = rotation_x;
                        player.rotation_y = rotation_y;
                    }
                }

                self.broadcast_message(
                    ServerMessage::PlayerMoved {
                        player_id: player_id.to_string(),
                        x, y, z, rotation_x, rotation_y
                    },
                    Some(player_id)
                ).await;
            }
            ClientMessage::Shoot { target_x, target_y, target_z } => {
                log::info!("Player {} shot at ({}, {}, {})", player_id, target_x, target_y, target_z);
                
                self.broadcast_message(
                    ServerMessage::PlayerShot {
                        shooter_id: player_id.to_string(),
                        target_x,
                        target_y,
                        target_z,
                    },
                    None
                ).await;

                // TODO: Add hit detection and damage calculation
            }
        }
    }

    async fn handle_connection(&self, stream: TcpStream, addr: SocketAddr) {
        let player_id = Uuid::new_v4().to_string();
        log::info!("New connection from {}, assigned ID: {}", addr, player_id);

        let ws_stream = match accept_async(stream).await {
            Ok(ws) => ws,
            Err(e) => {
                log::error!("Failed to accept WebSocket connection: {}", e);
                return;
            }
        };

        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<String>();
        self.connections.lock().unwrap().insert(player_id.clone(), tx);

        let welcome_msg = ServerMessage::Welcome {
            player_id: player_id.clone(),
        };
        
        let (mut ws_sender, ws_receiver) = ws_stream.split();
        
        let welcome_json = serde_json::to_string(&welcome_msg).unwrap();
        let _ = ws_sender.send(Message::Text(welcome_json)).await;

        
        tokio::spawn(async move {
            while let Some(msg) = rx.recv().await {
                if ws_sender.send(Message::Text(msg)).await.is_err() {
                    break;
                }
            }
        });

        self.handle_player_messages(player_id, ws_receiver).await;
    }

    async fn handle_player_messages(&self, player_id: String, mut ws_receiver: futures_util::stream::SplitStream<tokio_tungstenite::WebSocketStream<TcpStream>>) {
        while let Some(msg) = ws_receiver.next().await {
            match msg {
                Ok(Message::Text(text)) => {
                    if let Ok(client_msg) = serde_json::from_str::<ClientMessage>(&text) {
                        self.handle_client_message(&player_id, client_msg).await;
                    }
                }
                Ok(Message::Close(_)) | Err(_) => break,
                _ => {}
            }
        }

        self.connections.lock().unwrap().remove(&player_id);
        self.players.lock().unwrap().remove(&player_id);
        self.broadcast_message(
            ServerMessage::PlayerLeft { player_id: player_id.clone() },
            None
        ).await;

        log::info!("Player {} disconnected", player_id);
    }
}

#[tokio::main]
async fn main() {
    env_logger::init();

    let server = Arc::new(GameServer::new());
    let listener = TcpListener::bind("127.0.0.1:8080").await.unwrap();
    
    log::info!("Game server listening on ws://127.0.0.1:8080");

    while let Ok((stream, addr)) = listener.accept().await {
        let server_clone = Arc::clone(&server);
        tokio::spawn(async move {
            server_clone.handle_connection(stream, addr).await;
        });
    }
}