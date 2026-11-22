//! Per-connection handshake and session handling.
//!
//! Phase 3 session shape: after auth the framed connection is **split** —
//! a writer task drains an outbound channel (session replies + broadcasts
//! from other players), while this task reads the wire and handles session
//! messages. Broadcasts from any connection therefore reach every client.

use std::time::{SystemTime, UNIX_EPOCH};

use futures::channel::mpsc::{self, UnboundedSender};
use futures::{SinkExt, Stream, StreamExt};
use melon_proto::{
    validate_chat, validate_username, ClientMessage, HandshakeRejectReason, ProtoError,
    ServerMessage, ServerSessionCodec, PROTOCOL_VERSION,
};
use thiserror::Error;
use tokio::net::TcpStream;
use tokio::time::timeout;
use tokio_util::codec::Framed;

use crate::world::WorldHandle;
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

/// Drive one connection: handshake → world stream → session loop.
pub async fn handle_connection(
    stream: TcpStream,
    cfg: ServerConfig,
    world: WorldHandle,
) -> Result<(), AuthError> {
    let mut framed = Framed::new(stream, ServerSessionCodec::new());
    // Reserve the player id up front — the client learns it in the
    // handshake reply so it can filter its own broadcasts.
    let player_id = world.next_player_id();

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
                player_id,
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
                    player_id,
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
            player_id,
            server_name: cfg.server_name.clone(),
            motd: cfg.motd.clone(),
            rejection: None,
        })
        .await?;
    tracing::info!(%username, "player authenticated");

    // --- split: writer task drains outbound, reader task drives the session ---
    let (outbound_tx, mut outbound_rx) = mpsc::unbounded::<ServerMessage>();
    world.register(player_id, &username, outbound_tx.clone());
    let client_id = player_id;
    let (mut sink, mut wire_stream) = framed.split();

    let writer = tokio::spawn(async move {
        while let Some(msg) = outbound_rx.next().await {
            if sink.send(msg).await.is_err() {
                break;
            }
        }
    });

    // Post-auth payload: roster (excluding self), world, then announce the
    // join to everyone.
    let _ = outbound_tx.unbounded_send(ServerMessage::PlayerList {
        players: world.players_excluding(player_id),
    });
    for msg in world.initial_messages(player_id) {
        let _ = outbound_tx.unbounded_send(msg);
    }
    world.broadcast(&ServerMessage::PlayerJoin {
        id: player_id,
        name: username.clone(),
    });
    tracing::info!(%username, player_id, "world streamed");

    let result = session_loop(&mut wire_stream, &world, &outbound_tx, &username, player_id).await;

    world.broadcast(&ServerMessage::PlayerLeave { id: player_id });
    world.unregister(client_id);
    drop(outbound_tx); // writer drains the channel and finishes
    let _ = writer.await;

    match result {
        Ok(()) => {
            tracing::info!(%username, "session closed");
            Ok(())
        }
        Err(err) => {
            tracing::debug!(%username, error = %err, "connection ended");
            Err(err)
        }
    }
}

/// The authenticated session: read messages, handle them, reply via `out_tx`.
async fn session_loop<S>(
    wire_stream: &mut S,
    world: &WorldHandle,
    out_tx: &UnboundedSender<ServerMessage>,
    username: &str,
    player_id: u64,
) -> Result<(), AuthError>
where
    S: Stream<Item = Result<ClientMessage, ProtoError>> + Unpin,
{
    // Movement relay throttle: at most one PlayerMove per player per 50 ms
    // (clients send at ~10 Hz; this guards against abusive rates).
    let mut last_move: Option<std::time::Instant> = None;
    // Chat throttle: max 5 messages per 10 seconds per connection.
    let mut chat_window = std::time::Instant::now();
    let mut chat_count = 0u32;

    while let Some(msg) = wire_stream.next().await.transpose()? {
        match msg {
            ClientMessage::Ping {
                payload,
                client_timestamp_ms,
            } => {
                let _ = out_tx.unbounded_send(ServerMessage::Pong {
                    payload,
                    client_timestamp_ms,
                    server_timestamp_ms: now_ms(),
                });
            }
            ClientMessage::SetBlock { x, y, z, block } => {
                if let Err(err) = world.apply_edit(x, y, z, block) {
                    tracing::debug!(%username, error = %err, "edit rejected");
                    // Rejected edits are logged, not fatal — the session
                    // stays alive. No BlockUpdate means the edit didn't land.
                }
            }
            ClientMessage::Move(pos) => {
                let now = std::time::Instant::now();
                let throttled = last_move
                    .map(|t| now.duration_since(t) < std::time::Duration::from_millis(50))
                    .unwrap_or(false);
                if !throttled {
                    last_move = Some(now);
                    world.broadcast(&ServerMessage::PlayerMove {
                        id: player_id,
                        pos: [pos.x, pos.y, pos.z],
                        yaw: pos.yaw,
                        pitch: pos.pitch,
                    });
                }
            }
            ClientMessage::Chat { text } => {
                let now = std::time::Instant::now();
                if now.duration_since(chat_window) > std::time::Duration::from_secs(10) {
                    chat_window = now;
                    chat_count = 0;
                }
                chat_count += 1;
                if chat_count > 5 {
                    tracing::debug!(%username, "chat rate limit hit");
                    continue;
                }
                match validate_chat(&text) {
                    Ok(text) => {
                        world.broadcast(&ServerMessage::Chat {
                            id: player_id,
                            name: username.to_string(),
                            text,
                        });
                    }
                    Err(reason) => {
                        tracing::debug!(%username, %reason, "chat rejected");
                    }
                }
            }
            handshake @ ClientMessage::Handshake(_) => {
                tracing::warn!("duplicate handshake; dropping connection");
                return Err(AuthError::NotHandshake(Some(Box::new(handshake))));
            }
        }
    }
    Ok(())
}
