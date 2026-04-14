# RSGO Backend

<p align="center">
  <a href="https://github.com/rust-lang/cargo">
    <img src="https://img.shields.io/badge/cargo-project-orange?style=flat&logo=rust" alt="Cargo Project" />
  </a>
  <a href="https://tokio.rs/">
    <img src="https://img.shields.io/badge/async-tokio-blue?style=flat" alt="Async with Tokio" />
  </a>
  <a href="https://github.com/snapview/tokio-tungstenite">
    <img src="https://img.shields.io/badge/websocket-tungstenite-green?style=flat" alt="WebSocket with Tungstenite" />
  </a>
  <a href="https://github.com/serde-rs/serde">
    <img src="https://img.shields.io/badge/serialization-serde-red?style=flat" alt="Serialization with Serde" />
  </a>
  <br />
  <a href="https://github.com/uuid-rs/uuid">
    <img src="https://img.shields.io/badge/ids-uuid-purple?style=flat" alt="IDs with UUID" />
  </a>
  <a href="https://github.com/rust-lang/log">
    <img src="https://img.shields.io/badge/logging-log-yellow?style=flat" alt="Logging" />
  </a>
</p>

Multiplayer FPS game server built from scratch in Rust. Handles real-time player connections, game state, and combat mechanics over WebSocket.

## Architecture

**Core Server** (`src/core/`)
- Game server orchestration and connection handling
- Periodic tasks for shield regeneration and scoreboard updates

**Network Layer** (`src/network/`)
- WebSocket connection management
- JSON message protocol for client communication
- Message broadcasting to all connected players

**Game Logic** (`src/game/`)
- Player state management (health, shields, position, kills)
- Combat system with damage calculation
- Spawn system for player positioning
- Message handling for game events

**Player Management**
- Health/shield system with regeneration mechanics
- Death/respawn cycle with proper state validation
- Kill tracking and scoreboard maintenance

Runs on port 6969 with async Rust for handling concurrent player connections.