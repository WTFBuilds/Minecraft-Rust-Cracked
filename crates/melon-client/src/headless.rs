//! Headless client: the protocol session without a window. Used for the
//! Phase 1 acceptance demo, CI smoke tests, and later as the base for
//! scripted load-test bots.

use std::time::{Instant, SystemTime, UNIX_EPOCH};

use futures::{SinkExt, StreamExt};
use melon_proto::{ClientMessage, ClientSessionCodec, Handshake, ServerMessage, PROTOCOL_VERSION};
use tokio::net::TcpStream;
use tokio_util::codec::Framed;

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

/// Connect, authenticate, and run a ping loop — the Phase 1 session spine.
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

    println!("done — {pings} pings, session healthy");
    Ok(())
}
