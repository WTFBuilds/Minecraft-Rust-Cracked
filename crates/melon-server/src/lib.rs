//! MELON lightweight game server.
//!
//! Phase 1 implements the session spine: a TCP listener that speaks the
//! versioned MELON protocol — handshake validation, username auth, and
//! ping/pong. World streaming arrives with the protocol work in Phase 3.

pub mod conn;
pub mod server;

pub use conn::{handle_connection, AuthError};
pub use server::{bind, serve, ServerConfig};
