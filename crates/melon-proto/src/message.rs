//! Message types for protocol version 1.
//!
//! Phase 1 covers the session spine: versioned handshake, auth result, and
//! ping/pong. Gameplay messages (world, movement, combat) join in later
//! phases — the enums are designed to grow without breaking framing.

use serde::{Deserialize, Serialize};

/// Wire protocol version. The server rejects any other version with an
/// explicit reason instead of guessing.
pub const PROTOCOL_VERSION: u16 = 1;

/// Hard cap on a single frame payload (protects against malicious length
/// prefixes). 1 MiB is generous for Phase 1; chunk payloads in Phase 3 stay
/// well under it.
pub const MAX_FRAME_SIZE: u32 = 1024 * 1024;

/// Maximum username length the server accepts.
pub const MAX_USERNAME_LEN: usize = 16;

/// First message a client sends on every new connection.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Handshake {
    /// Must equal [`PROTOCOL_VERSION`].
    pub protocol_version: u16,
    /// Human-readable client build string, e.g. `"melon-client 0.1.0"`.
    pub client_build: String,
    /// Requested player name (validated server-side).
    pub username: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum HandshakeRejectReason {
    ProtocolVersionMismatch { expected: u16, got: u16 },
    InvalidUsername { reason: String },
    ServerFull,
}

/// One 16×16×16 section packed for the wire (see `melon-world`'s
/// `encode_sections`): non-empty sections only, LE `u16` block ids.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChunkData {
    pub cx: i32,
    pub cz: i32,
    /// Bit `i` set = section `i` is present in `sections`.
    pub section_mask: u16,
    pub sections: Vec<u8>,
}

/// Player movement: position + look direction, client-authoritative with
/// server-side clamping to come in Phase 6.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct PlayerMove {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub yaw: f32,
    pub pitch: f32,
}

/// Messages sent by the client.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ClientMessage {
    Handshake(Handshake),
    /// Connectivity probe. `client_timestamp_ms` is echoed untouched so the
    /// client can compute RTT against its own clock.
    Ping {
        payload: u64,
        client_timestamp_ms: u64,
    },
    /// Player edits a block. `block: 0` means break. The server validates
    /// and broadcasts a [`ServerMessage::BlockUpdate`] on success.
    SetBlock {
        x: i32,
        y: i32,
        z: i32,
        block: u16,
    },
    /// Current position + look (throttled by the client to ~10 Hz).
    Move(PlayerMove),
    /// A chat line. The server validates, rate-limits, stamps the sender's
    /// name, and broadcasts [`ServerMessage::Chat`].
    Chat {
        text: String,
    },
}

/// Where the server drops a player after auth: position + look direction
/// (yaw/pitch are client-side defaults for now — presence sync lands in
/// Phase 4).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WorldInfo {
    pub spawn: [f32; 3],
    /// How many `ChunkData` messages make up the full world.
    pub chunks: u32,
}

/// A connected player, for the post-auth roster.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlayerEntry {
    pub id: u64,
    pub name: String,
}

/// Messages sent by the server.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ServerMessage {
    /// Exactly one per connection, in response to the [`Handshake`].
    HandshakeResult {
        accepted: bool,
        /// The protocol version the server is running.
        protocol_version: u16,
        /// This client's player id — used to filter its own broadcasts.
        player_id: u64,
        server_name: String,
        motd: String,
        rejection: Option<HandshakeRejectReason>,
    },
    Pong {
        payload: u64,
        client_timestamp_ms: u64,
        server_timestamp_ms: u64,
    },
    /// First world message after auth: spawn point + chunk count.
    WorldInfo(WorldInfo),
    /// One world chunk: non-empty sections packed LE.
    ChunkData(ChunkData),
    /// A block changed (by this or another player). Sent to every client.
    BlockUpdate { x: i32, y: i32, z: i32, block: u16 },
    /// Roster snapshot right after auth (before this player's Join is
    /// broadcast): everyone already in the world.
    PlayerList { players: Vec<PlayerEntry> },
    /// A player joined (id ≠ this client's own).
    PlayerJoin { id: u64, name: String },
    /// A player left.
    PlayerLeave { id: u64 },
    /// A player moved (relayed, server-throttled).
    PlayerMove {
        id: u64,
        pos: [f32; 3],
        yaw: f32,
        pitch: f32,
    },
    /// A chat line, stamped with the sender's id + name.
    Chat { id: u64, name: String, text: String },
}

/// Maximum chat message length.
pub const MAX_CHAT_LEN: usize = 200;

/// Server-side chat validation shared by the connection handler.
pub fn validate_chat(raw: &str) -> Result<String, String> {
    let text = raw.trim();
    if text.is_empty() {
        return Err("chat message is empty".to_string());
    }
    if text.chars().any(char::is_control) {
        return Err("chat message contains control characters".to_string());
    }
    if text.chars().count() > MAX_CHAT_LEN {
        return Err(format!(
            "chat message longer than {MAX_CHAT_LEN} characters"
        ));
    }
    Ok(text.to_string())
}

/// Server-side username validation shared by the connection handler.
pub fn validate_username(raw: &str) -> Result<&str, String> {
    let name = raw.trim();
    if name.is_empty() {
        return Err("username is empty".to_string());
    }
    if name.len() > MAX_USERNAME_LEN {
        return Err(format!(
            "username longer than {MAX_USERNAME_LEN} characters"
        ));
    }
    if !name
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
    {
        return Err("username may only contain letters, digits, '_' and '-'".to_string());
    }
    Ok(name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn handshake_message_roundtrip() {
        let msg = ClientMessage::Handshake(Handshake {
            protocol_version: PROTOCOL_VERSION,
            client_build: "melon-client 0.1.0".to_string(),
            username: "Ahqaf".to_string(),
        });
        let bytes =
            bincode::serde::encode_to_vec(&msg, bincode::config::standard()).expect("encode");
        let (decoded, _): (ClientMessage, _) =
            bincode::serde::decode_from_slice(&bytes, bincode::config::standard()).expect("decode");
        assert_eq!(decoded, msg);
    }

    #[test]
    fn ping_pong_roundtrip() {
        let ping = ClientMessage::Ping {
            payload: 0xDEAD_BEEF,
            client_timestamp_ms: 1_234_567,
        };
        let bytes =
            bincode::serde::encode_to_vec(&ping, bincode::config::standard()).expect("encode");
        let (decoded, _): (ClientMessage, _) =
            bincode::serde::decode_from_slice(&bytes, bincode::config::standard()).expect("decode");
        assert_eq!(decoded, ping);

        let pong = ServerMessage::Pong {
            payload: 0xDEAD_BEEF,
            client_timestamp_ms: 1_234_567,
            server_timestamp_ms: 1_234_570,
        };
        let bytes =
            bincode::serde::encode_to_vec(&pong, bincode::config::standard()).expect("encode");
        let (decoded, _): (ServerMessage, _) =
            bincode::serde::decode_from_slice(&bytes, bincode::config::standard()).expect("decode");
        assert_eq!(decoded, pong);
    }

    #[test]
    fn rejection_reasons_roundtrip() {
        let msg = ServerMessage::HandshakeResult {
            accepted: false,
            protocol_version: PROTOCOL_VERSION,
            player_id: 0,
            server_name: "MELON".to_string(),
            motd: String::new(),
            rejection: Some(HandshakeRejectReason::ProtocolVersionMismatch {
                expected: PROTOCOL_VERSION,
                got: 99,
            }),
        };
        let bytes =
            bincode::serde::encode_to_vec(&msg, bincode::config::standard()).expect("encode");
        let (decoded, _): (ServerMessage, _) =
            bincode::serde::decode_from_slice(&bytes, bincode::config::standard()).expect("decode");
        assert_eq!(decoded, msg);
    }

    #[test]
    fn username_validation() {
        assert_eq!(validate_username("Ahqaf_99"), Ok("Ahqaf_99"));
        assert!(validate_username("").is_err());
        assert!(validate_username("   ").is_err());
        assert!(validate_username("has space").is_err());
        assert!(validate_username("12345678901234567").is_err()); // 17 chars
        assert_eq!(validate_username(" trimmed  "), Ok("trimmed"));
    }

    #[test]
    fn phase3_messages_roundtrip() {
        let world_info = ServerMessage::WorldInfo(WorldInfo {
            spawn: [8.5, 12.0, 56.0],
            chunks: 16,
        });
        let chunk = ServerMessage::ChunkData(ChunkData {
            cx: 1,
            cz: 2,
            section_mask: 0b11,
            sections: vec![1, 2, 3, 4, 5],
        });
        let set_block = ClientMessage::SetBlock {
            x: 3,
            y: 6,
            z: 4,
            block: 9,
        };
        let block_update = ServerMessage::BlockUpdate {
            x: 3,
            y: 6,
            z: 4,
            block: 9,
        };

        for msg in [world_info, chunk, block_update] {
            let bytes =
                bincode::serde::encode_to_vec(&msg, bincode::config::standard()).expect("encode");
            let (decoded, _): (ServerMessage, _) =
                bincode::serde::decode_from_slice(&bytes, bincode::config::standard())
                    .expect("decode");
            assert_eq!(decoded, msg);
        }
        let bytes =
            bincode::serde::encode_to_vec(&set_block, bincode::config::standard()).expect("encode");
        let (decoded, _): (ClientMessage, _) =
            bincode::serde::decode_from_slice(&bytes, bincode::config::standard()).expect("decode");
        assert_eq!(decoded, set_block);
    }

    #[test]
    fn phase4_messages_roundtrip() {
        let accepted = ServerMessage::HandshakeResult {
            accepted: true,
            protocol_version: PROTOCOL_VERSION,
            player_id: 42,
            server_name: "MELON".to_string(),
            motd: String::new(),
            rejection: None,
        };
        let roster = ServerMessage::PlayerList {
            players: vec![PlayerEntry {
                id: 1,
                name: "Alice".to_string(),
            }],
        };
        let join = ServerMessage::PlayerJoin {
            id: 2,
            name: "Bob".to_string(),
        };
        let leave = ServerMessage::PlayerLeave { id: 2 };
        let moved = ServerMessage::PlayerMove {
            id: 1,
            pos: [8.5, 12.0, 56.0],
            yaw: -1.57,
            pitch: -0.4,
        };
        let chat = ServerMessage::Chat {
            id: 1,
            name: "Alice".to_string(),
            text: "hello world".to_string(),
        };
        let move_in = ClientMessage::Move(PlayerMove {
            x: 8.5,
            y: 12.0,
            z: 56.0,
            yaw: -1.57,
            pitch: -0.4,
        });
        let chat_in = ClientMessage::Chat {
            text: "hi".to_string(),
        };

        for msg in [accepted, roster, join, leave, moved, chat] {
            let bytes =
                bincode::serde::encode_to_vec(&msg, bincode::config::standard()).expect("encode");
            let (decoded, _): (ServerMessage, _) =
                bincode::serde::decode_from_slice(&bytes, bincode::config::standard())
                    .expect("decode");
            assert_eq!(decoded, msg);
        }
        for msg in [move_in, chat_in] {
            let bytes =
                bincode::serde::encode_to_vec(&msg, bincode::config::standard()).expect("encode");
            let (decoded, _): (ClientMessage, _) =
                bincode::serde::decode_from_slice(&bytes, bincode::config::standard())
                    .expect("decode");
            assert_eq!(decoded, msg);
        }
    }
}
