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
    let port = std::env::var("PORT").unwrap_or_else(|_| "6969".to_string());
    let bind_addr = format!("0.0.0.0:{}", port);
    let listener = TcpListener::bind(&bind_addr).await.unwrap();
    
    log::info!("Game server listening on ws://0.0.0.0:{}", port);

    while let Ok((stream, addr)) = listener.accept().await {
        let server_clone = Arc::clone(&server);
        tokio::spawn(async move {
            server_clone.handle_connection(stream, addr).await;
        });
    }
}