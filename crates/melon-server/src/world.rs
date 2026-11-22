//! The server-owned world: shared state, edit validation, and broadcast.
//!
//! Phase 3 keeps the dev world under a single `WorldHandle` (a `World` behind
//! a mutex + a sender registry). The "one actor per match" rule still holds
//! for the match runtime in Phase 8 — this is the shared lobby-style world
//! every client connects into for now.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use futures::channel::mpsc::UnboundedSender;
use melon_proto::{ChunkData, PlayerEntry, ServerMessage, WorldInfo};
use melon_world::{block, FlatGenerator, World, CHUNK_H, CHUNK_W};
use thiserror::Error;

/// A connected client's outbound channel + identity, registered after auth.
struct Registered {
    id: u64,
    name: String,
    tx: UnboundedSender<ServerMessage>,
}

/// Shared world + per-connection broadcast registry. `Clone` shares state.
#[derive(Clone)]
pub struct WorldHandle {
    world: Arc<Mutex<World>>,
    size_chunks: u32,
    senders: Arc<Mutex<HashMap<u64, Registered>>>,
    next_id: Arc<Mutex<u64>>,
}

#[derive(Debug, Error)]
pub enum EditError {
    #[error("block y {0} is outside the world height (0..{MAX_Y})")]
    OutOfHeight(i32),
    #[error("block ({x}, {z}) is outside the {size}×{size}-chunk world")]
    OutOfBounds { x: i32, z: i32, size: u32 },
}

const MAX_Y: i32 = CHUNK_H as i32;

impl WorldHandle {
    /// Generate the superflat dev world (`size_chunks × size_chunks`).
    pub fn new(size_chunks: u32) -> Self {
        Self {
            world: Arc::new(Mutex::new(
                FlatGenerator::superflat().generate(size_chunks, size_chunks),
            )),
            size_chunks,
            senders: Arc::new(Mutex::new(HashMap::new())),
            next_id: Arc::new(Mutex::new(0)),
        }
    }

    pub fn size_chunks(&self) -> u32 {
        self.size_chunks
    }

    /// Blocks per world edge.
    pub fn size_blocks(&self) -> i32 {
        self.size_chunks as i32 * CHUNK_W as i32
    }

    /// Spawn: eye position looking across the world (the client applies its
    /// own default yaw/pitch). Offset per player id so simultaneous joins
    /// don't stack inside each other.
    pub fn spawn_for(&self, id: u64) -> [f32; 3] {
        let offset = ((id % 8) * 3) as f32;
        // Feet position: resting on the grass top (y=6).
        [8.5 + offset, 7.0, (self.size_blocks() - 8).max(1) as f32]
    }

    /// Reserve the next player id (called before the handshake reply so the
    /// client learns its own id immediately).
    pub fn next_player_id(&self) -> u64 {
        let mut ids = self.next_id.lock().unwrap();
        *ids += 1;
        *ids
    }

    /// Register a post-auth client (with its pre-assigned id) for broadcasts.
    pub fn register(&self, id: u64, name: &str, tx: UnboundedSender<ServerMessage>) {
        self.senders.lock().unwrap().insert(
            id,
            Registered {
                id,
                name: name.to_string(),
                tx,
            },
        );
    }

    /// The current roster (everyone except `except_id`).
    pub fn players_excluding(&self, except_id: u64) -> Vec<PlayerEntry> {
        self.senders
            .lock()
            .unwrap()
            .values()
            .filter(|r| r.id != except_id)
            .map(|r| PlayerEntry {
                id: r.id,
                name: r.name.clone(),
            })
            .collect()
    }

    /// Remove a client on disconnect.
    pub fn unregister(&self, id: u64) {
        self.senders.lock().unwrap().remove(&id);
    }

    /// Fan out to every connected client, pruning dead channels.
    pub fn broadcast(&self, msg: &ServerMessage) {
        let mut senders = self.senders.lock().unwrap();
        let dead: Vec<u64> = senders
            .values()
            .filter(|r| r.tx.unbounded_send(msg.clone()).is_err())
            .map(|r| r.id)
            .collect();
        for id in dead {
            senders.remove(&id);
        }
    }

    /// Build the post-auth world payload for one player: `WorldInfo` +
    /// every `ChunkData` (roster is sent separately by the connection).
    pub fn initial_messages(&self, id: u64) -> Vec<ServerMessage> {
        let world = self.world.lock().unwrap();
        let mut msgs = vec![ServerMessage::WorldInfo(WorldInfo {
            spawn: self.spawn_for(id),
            chunks: world.chunk_count() as u32,
        })];
        for (&(cx, cz), chunk) in world.chunks() {
            let (section_mask, sections) = chunk.encode_sections();
            msgs.push(ServerMessage::ChunkData(ChunkData {
                cx,
                cz,
                section_mask,
                sections,
            }));
        }
        msgs
    }

    /// Validate + apply a block edit, then broadcast the update.
    /// Returns `Err` with a client-friendly reason on rejection.
    pub fn apply_edit(&self, x: i32, y: i32, z: i32, block: u16) -> Result<(), EditError> {
        if !(0..MAX_Y).contains(&y) {
            return Err(EditError::OutOfHeight(y));
        }
        let limit = self.size_blocks();
        if x < 0 || x >= limit || z < 0 || z >= limit {
            return Err(EditError::OutOfBounds {
                x,
                z,
                size: self.size_chunks,
            });
        }

        let was_air;
        {
            let mut world = self.world.lock().unwrap();
            was_air = world.get_block(x, y, z).unwrap_or(block::AIR) == block::AIR;
            world.set_block(x, y, z, block);
        }
        let _ = was_air; // recorded for future anti-grief metrics

        self.broadcast(&ServerMessage::BlockUpdate { x, y, z, block });
        Ok(())
    }
}
