<h1 align="center">RSGO&nbsp;·&nbsp;Backend</h1>

<p align="center"><b>The authoritative game server.</b><br/>
Real-time multiplayer, written from scratch in Rust.</p>

<p align="center"><a href="https://rsgo.io"><b>Play at rsgo.io →</b></a></p>

<p align="center">
  <img src="https://img.shields.io/badge/-%23ef4e23-ef4e23?style=flat&label=RSGO&labelColor=1a2447" alt="RSGO" />
  <img src="https://img.shields.io/badge/built%20in-rust-1a2447?style=flat&logo=rust&labelColor=ef4e23" alt="Rust" />
  <img src="https://img.shields.io/badge/async-tokio-1a2447?style=flat&labelColor=11182f" alt="Tokio" />
  <img src="https://img.shields.io/badge/transport-websocket-1a2447?style=flat&labelColor=11182f" alt="WebSocket" />
  <img src="https://img.shields.io/badge/serialization-serde-1a2447?style=flat&labelColor=11182f" alt="Serde" />
</p>

---

## What is RSGO?

A competitive 3D multiplayer FPS with one twist: **you don't buy guns, you buy the map.** Each round the players build the battlefield themselves, so there's nothing fixed to memorise. **Skill over study.**

This repository is the **game server** — the single source of truth for every match. Clients send inputs; the server decides what's real: positions, damage, deaths, economy, the round and bomb state machine, and who won.

## What it handles

- **Real-time connections** — async WebSocket, many players at once, JSON message protocol.
- **Authoritative state** — health & shields, hits & deaths, kills & scoreboard, all validated server-side.
- **Round lifecycle** — waiting → build phase → play → round end, with team switch and match end.
- **Bomb mechanics** — plant, defuse, explosion timers and win conditions.
- **In-memory & fast** — lobbies live in memory; no database in the loop.

## Under the hood

| Area | What lives here |
|------|-----------------|
| `src/core/` | Server orchestration & the periodic game tick |
| `src/network/` | WebSocket handling, protocol, broadcasting |
| `src/game/` | Lobbies, rounds, players, combat, spawns, collision |

Built on **Tokio** async, **tokio-tungstenite** for WebSocket, and **Serde** for the JSON protocol shared with the client.
