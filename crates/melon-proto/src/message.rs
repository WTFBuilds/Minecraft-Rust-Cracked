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

/// Messages sent by the client.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ClientMessage {
    Handshake(Handshake),
    /// Connectivity probe. `client_timestamp_ms` is echoed untouched so the
    /// client can compute RTT against its own clock.
    Ping {
        payload: u64,
        client_timestamp_ms: u64,
    },
}

/// Messages sent by the server.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ServerMessage {
    /// Exactly one per connection, in response to the [`Handshake`].
    HandshakeResult {
        accepted: bool,
        /// The protocol version the server is running.
        protocol_version: u16,
        server_name: String,
        motd: String,
        rejection: Option<HandshakeRejectReason>,
    },
    Pong {
        payload: u64,
        client_timestamp_ms: u64,
        server_timestamp_ms: u64,
    },
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
}
