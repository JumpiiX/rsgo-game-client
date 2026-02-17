mod core;
mod network;
mod game;
mod utils;

use crate::core::GameServer;
use tokio::net::TcpListener;
use std::sync::Arc;

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