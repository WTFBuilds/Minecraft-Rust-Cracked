//! MELON lightweight game server.
//!
//! Phase 3: the server owns the world — after auth it streams every chunk,
//! then serves validated `SetBlock` edits broadcast to all connected clients.

pub mod conn;
pub mod server;
pub mod world;

pub use conn::{handle_connection, AuthError};
pub use server::{bind, serve, ServerConfig};
pub use world::WorldHandle;
