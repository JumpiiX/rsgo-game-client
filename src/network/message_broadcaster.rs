use crate::network::ServerMessage;
use std::collections::HashMap;
use std::sync::Mutex;
use tokio::sync::mpsc;

pub struct MessageBroadcaster {
    connections: Mutex<HashMap<String, mpsc::UnboundedSender<String>>>,
}

impl MessageBroadcaster {
    pub fn new() -> Self {
        Self {
            connections: Mutex::new(HashMap::new()),
        }
    }

    pub fn add_connection(&self, player_id: String, sender: mpsc::UnboundedSender<String>) {
        self.connections.lock().unwrap().insert(player_id, sender);
    }

    pub fn remove_connection(&self, player_id: &str) {
        self.connections.lock().unwrap().remove(player_id);
    }

    pub async fn broadcast_message(&self, message: ServerMessage, exclude_id: Option<&str>) {
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

    pub fn send_to_player(&self, player_id: &str, message: ServerMessage) {
        let connections = self.connections.lock().unwrap();
        if let Some(sender) = connections.get(player_id) {
            let msg_json = serde_json::to_string(&message).unwrap();
            let _ = sender.send(msg_json);
        }
    }
}