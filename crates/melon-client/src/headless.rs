//! Headless client: the protocol session without a window. Used for the
//! acceptance demos, CI smoke tests, and later as the base for scripted
//! load-test bots.
//!
//! Phase 3 flow: auth → receive the streamed world → pings → place a block
//! and watch it echo back.

use std::time::{Instant, SystemTime, UNIX_EPOCH};

use futures::{SinkExt, StreamExt};
use melon_proto::{ClientMessage, ClientSessionCodec, Handshake, ServerMessage, PROTOCOL_VERSION};
use melon_world::block;
use tokio::net::TcpStream;
use tokio_util::codec::Framed;

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

/// Connect, authenticate, receive the world, ping, and prove block edits.
pub async fn run(
    host: &str,
    port: u16,
    username: &str,
    pings: u32,
) -> Result<(), Box<dyn std::error::Error>> {
    let tcp = TcpStream::connect((host, port)).await?;
    println!("connected to {host}:{port}");
    let mut framed = Framed::new(tcp, ClientSessionCodec::new());

    framed
        .send(ClientMessage::Handshake(Handshake {
            protocol_version: PROTOCOL_VERSION,
            client_build: format!("melon-client {}", env!("CARGO_PKG_VERSION")),
            username: username.to_string(),
        }))
        .await?;

    let reply = framed
        .next()
        .await
        .ok_or("server closed before handshake result")??;

    match reply {
        ServerMessage::HandshakeResult {
            accepted,
            protocol_version,
            player_id: _,
            server_name,
            motd,
            rejection,
        } => {
            if !accepted {
                println!(
                    "handshake rejected by {server_name} (server protocol {protocol_version}): {rejection:?}"
                );
                return Ok(());
            }
            println!("handshake accepted — server: {server_name} · protocol {protocol_version}");
            println!("motd: {motd}");
        }
        other => return Err(format!("unexpected message: {other:?}").into()),
    }

    // --- Phase 3: world stream ---
    let mut expected_chunks = 0u32;
    let mut received_chunks = 0u32;
    let mut chunk_bytes = 0usize;
    loop {
        let msg = framed
            .next()
            .await
            .ok_or("server closed during world stream")??;
        match msg {
            ServerMessage::WorldInfo(info) => {
                expected_chunks = info.chunks;
                let [sx, sy, sz] = info.spawn;
                println!("world: spawn ({sx:.1}, {sy:.1}, {sz:.1}) · {expected_chunks} chunks");
                if expected_chunks == 0 {
                    break;
                }
            }
            ServerMessage::ChunkData(chunk) => {
                received_chunks += 1;
                chunk_bytes += chunk.sections.len();
                if received_chunks == expected_chunks {
                    println!(
                        "world received: {received_chunks} chunks · {chunk_bytes} bytes of sections"
                    );
                    break;
                }
            }
            other => {
                return Err(format!("unexpected message during world stream: {other:?}").into())
            }
        }
    }

    // --- pings ---
    for i in 0..pings {
        let sent = Instant::now();
        framed
            .send(ClientMessage::Ping {
                payload: u64::from(i),
                client_timestamp_ms: now_ms(),
            })
            .await?;
        match framed.next().await.ok_or("server closed mid-ping")?? {
            ServerMessage::Pong { payload, .. } => {
                assert_eq!(payload, u64::from(i), "pong payload must echo the ping");
                println!("ping #{i}: rtt = {:.2?}", sent.elapsed());
            }
            other => return Err(format!("unexpected message during ping loop: {other:?}").into()),
        }
    }

    // --- Phase 3: block edit → server broadcast echo ---
    let edit_pos = [2i32, 6, 2];
    framed
        .send(ClientMessage::SetBlock {
            x: edit_pos[0],
            y: edit_pos[1],
            z: edit_pos[2],
            block: block::WOOL_RED,
        })
        .await?;
    let msg = framed
        .next()
        .await
        .ok_or("server closed before block update")??;
    match msg {
        ServerMessage::BlockUpdate { x, y, z, block } => {
            assert_eq!(
                (x, y, z, block),
                (edit_pos[0], edit_pos[1], edit_pos[2], block::WOOL_RED),
                "our own edit must echo back"
            );
            println!("block edit echoed — set ({x}, {y}, {z}) to wool:red ✔");
        }
        other => return Err(format!("unexpected message during edit: {other:?}").into()),
    }

    println!("done — world streamed, {pings} pings, block edit verified");
    Ok(())
}
