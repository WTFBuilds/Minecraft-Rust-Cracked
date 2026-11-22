//! Listener lifecycle.

use std::io;
use std::net::SocketAddr;

use tokio::net::TcpListener;

use crate::conn::handle_connection;
use crate::world::WorldHandle;

/// Static server settings, passed to [`bind`] and every connection.
#[derive(Debug, Clone)]
pub struct ServerConfig {
    /// Listen address; `127.0.0.1:0` lets the OS pick a port (used by tests).
    pub bind_addr: SocketAddr,
    pub server_name: String,
    pub motd: String,
    /// World side length in chunks (`N` → an N×N-chunk superflat).
    pub world_chunks: u32,
}

/// Bind a listener for the given config. Returns the bound address so tests
/// and embedders can use an OS-assigned port (`127.0.0.1:0`).
pub async fn bind(cfg: &ServerConfig) -> io::Result<(TcpListener, std::net::SocketAddr)> {
    let listener = TcpListener::bind(cfg.bind_addr).await?;
    let addr = listener.local_addr()?;
    tracing::info!(
        %addr,
        name = %cfg.server_name,
        world_chunks = cfg.world_chunks,
        "MELON server listening"
    );
    Ok((listener, addr))
}

/// Accept loop: one task per connection sharing the world handle. Errors on
/// a single connection are logged and dropped — a bad client can never take
/// the server down.
pub async fn serve(listener: TcpListener, cfg: ServerConfig, world: WorldHandle) -> io::Result<()> {
    loop {
        let (stream, peer) = listener.accept().await?;
        tracing::debug!(%peer, "connection accepted");
        let conn_cfg = cfg.clone();
        let conn_world = world.clone();
        tokio::spawn(async move {
            if let Err(err) = handle_connection(stream, conn_cfg, conn_world).await {
                tracing::debug!(%peer, error = %err, "connection ended");
            }
        });
    }
}
