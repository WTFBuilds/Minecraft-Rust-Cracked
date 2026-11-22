//! MELON platform wire protocol.
//!
//! Everything on the wire is a **frame**: a VarInt length prefix followed by a
//! bincode-encoded message payload. Every session starts with a versioned
//! handshake; codecs refuse mismatched protocol versions loudly.
//!
//! Perspective codecs pin the direction: [`ServerSessionCodec`] is what the
//! server runs on a connection, [`ClientSessionCodec`] is what the client
//! runs. They cannot be mixed up.

pub mod error;
pub mod frame;
pub mod message;
pub mod varint;

pub use error::ProtoError;
pub use frame::{ClientSessionCodec, ServerSessionCodec};
pub use message::{
    validate_chat, validate_username, ChunkData, ClientMessage, Handshake, HandshakeRejectReason,
    PlayerEntry, PlayerMove, ServerMessage, WorldInfo, PROTOCOL_VERSION,
};
pub use varint::{read_varint, varint_bytes, VarIntError};
