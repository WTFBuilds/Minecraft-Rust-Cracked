//! Per-connection handshake and session handling.

use std::time::{SystemTime, UNIX_EPOCH};

use futures::{SinkExt, StreamExt};
use melon_proto::{
    validate_username, ClientMessage, HandshakeRejectReason, ProtoError, ServerMessage,
    ServerSessionCodec, PROTOCOL_VERSION,
};
use thiserror::Error;
use tokio::net::TcpStream;
use tokio::time::timeout;
use tokio_util::codec::Framed;

use crate::ServerConfig;

const HANDSHAKE_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(10);

#[derive(Debug, Error)]
pub enum AuthError {
    #[error("non-handshake message before authentication: {0:?}")]
    NotHandshake(Option<Box<ClientMessage>>),
    #[error("handshake timed out")]
    TimedOut,
    #[error("protocol error: {0}")]
    Protocol(#[from] ProtoError),
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

/// Drive one connection from handshake through session messages.
///
/// Phase 1 session = ping/pong echo. Phase 3 replaces the loop body with the
/// world-streaming session; the handshake contract stays identical.
pub async fn handle_connection(stream: TcpStream, cfg: ServerConfig) -> Result<(), AuthError> {
    let mut framed = Framed::new(stream, ServerSessionCodec::new());

    // --- handshake: one message, bounded time ---
    let first = timeout(HANDSHAKE_TIMEOUT, framed.next())
        .await
        .map_err(|_| AuthError::TimedOut)?
        .transpose()?;

    let Some(ClientMessage::Handshake(handshake)) = first else {
        return Err(AuthError::NotHandshake(first.map(Box::new)));
    };

    tracing::info!(
        username = %handshake.username,
        client_version = handshake.protocol_version,
        build = %handshake.client_build,
        "handshake received"
    );

    if handshake.protocol_version != PROTOCOL_VERSION {
        framed
            .send(ServerMessage::HandshakeResult {
                accepted: false,
                protocol_version: PROTOCOL_VERSION,
                server_name: cfg.server_name.clone(),
                motd: String::new(),
                rejection: Some(HandshakeRejectReason::ProtocolVersionMismatch {
                    expected: PROTOCOL_VERSION,
                    got: handshake.protocol_version,
                }),
            })
            .await?;
        return Ok(());
    }

    let username = match validate_username(&handshake.username) {
        Ok(name) => name.to_string(),
        Err(reason) => {
            framed
                .send(ServerMessage::HandshakeResult {
                    accepted: false,
                    protocol_version: PROTOCOL_VERSION,
                    server_name: cfg.server_name.clone(),
                    motd: String::new(),
                    rejection: Some(HandshakeRejectReason::InvalidUsername { reason }),
                })
                .await?;
            return Ok(());
        }
    };

    framed
        .send(ServerMessage::HandshakeResult {
            accepted: true,
            protocol_version: PROTOCOL_VERSION,
            server_name: cfg.server_name.clone(),
            motd: cfg.motd.clone(),
            rejection: None,
        })
        .await?;
    tracing::info!(%username, "player authenticated");

    // --- session ---
    while let Some(msg) = framed.next().await.transpose()? {
        match msg {
            ClientMessage::Ping {
                payload,
                client_timestamp_ms,
            } => {
                framed
                    .send(ServerMessage::Pong {
                        payload,
                        client_timestamp_ms,
                        server_timestamp_ms: now_ms(),
                    })
                    .await?;
            }
            handshake @ ClientMessage::Handshake(_) => {
                // Re-handshake on a live session is a protocol violation —
                // never guess. Drop the connection.
                tracing::warn!("duplicate handshake; dropping connection");
                return Err(AuthError::NotHandshake(Some(Box::new(handshake))));
            }
        }
    }

    tracing::info!(%username, "session closed");
    Ok(())
}
