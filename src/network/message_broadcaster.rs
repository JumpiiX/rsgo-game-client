use crate::network::ServerMessage;
use std::collections::{HashMap, HashSet};
use std::sync::Mutex;
use tokio::sync::mpsc;

pub struct MessageBroadcaster {
    connections: Mutex<HashMap<String, mpsc::UnboundedSender<String>>>,
    lobby_players: Mutex<HashMap<String, HashSet<String>>>,
}

impl MessageBroadcaster {
    pub fn new() -> Self {
        Self {
            connections: Mutex::new(HashMap::new()),
            lobby_players: Mutex::new(HashMap::new()),
        }
    }

    pub fn add_connection(&self, player_id: String, sender: mpsc::UnboundedSender<String>) {
        self.connections.lock().unwrap().insert(player_id, sender);
    }

    pub fn remove_connection(&self, player_id: &str) {
        self.connections.lock().unwrap().remove(player_id);
    }

    pub fn broadcast_message(&self, message: &ServerMessage, exclude_id: Option<&str>) {
        let msg_json = serde_json::to_string(message).unwrap();
        let connections = self.connections.lock().unwrap().clone();

        for (id, sender) in &connections {
            if let Some(exclude) = exclude_id {
                if id == exclude {
                    continue;
                }
            }

            let _ = sender.send(msg_json.clone());
        }
    }

    pub fn send_to_player(&self, player_id: &str, message: &ServerMessage) {
        let connections = self.connections.lock().unwrap();
        if let Some(sender) = connections.get(player_id) {
            let msg_json = serde_json::to_string(message).unwrap();
            let _ = sender.send(msg_json);
        }
    }

    pub fn add_player_to_lobby(&self, lobby_id: &str, player_id: &str) {
        let mut lobby_players = self.lobby_players.lock().unwrap();
        lobby_players.entry(lobby_id.to_string())
            .or_insert_with(HashSet::new)
            .insert(player_id.to_string());
    }

    pub fn remove_player_from_lobby(&self, lobby_id: &str, player_id: &str) {
        let mut lobby_players = self.lobby_players.lock().unwrap();
        if let Some(players) = lobby_players.get_mut(lobby_id) {
            players.remove(player_id);
            if players.is_empty() {
                lobby_players.remove(lobby_id);
            }
        }
    }

    pub fn broadcast_to_lobby(&self, lobby_id: &str, message: &ServerMessage, exclude_id: Option<&str>) {
        let msg_json = serde_json::to_string(message).unwrap();
        let lobby_players = self.lobby_players.lock().unwrap();
        let connections = self.connections.lock().unwrap();

        if let Some(players) = lobby_players.get(lobby_id) {
            for player_id in players {
                if let Some(exclude) = exclude_id {
                    if player_id == exclude {
                        continue;
                    }
                }

                if let Some(sender) = connections.get(player_id) {
                    let _ = sender.send(msg_json.clone());
                }
            }
        }
    }
}