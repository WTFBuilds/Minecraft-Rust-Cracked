//! End-to-end protocol tests: real TCP round trips through the server —
//! the session spine (Phase 1), world streaming + block edits (Phase 3),
//! and presence/chat (Phase 4).

use std::time::Duration;

use futures::{SinkExt, StreamExt};
use melon_proto::{
    ChunkData, ClientMessage, ClientSessionCodec, Handshake, HandshakeRejectReason, PlayerEntry,
    ServerMessage, WorldInfo, PROTOCOL_VERSION,
};
use melon_server::{bind, serve, ServerConfig, WorldHandle};
use melon_world::block;
use tokio::net::TcpStream;
use tokio_util::codec::Framed;

fn test_config() -> ServerConfig {
    ServerConfig {
        bind_addr: "127.0.0.1:0".parse().expect("test addr is valid"),
        server_name: "MELON-test".to_string(),
        motd: "hello from the test server".to_string(),
        world_chunks: 2,
    }
}

async fn spawn_server() -> std::net::SocketAddr {
    let cfg = test_config();
    let world = WorldHandle::new(cfg.world_chunks);
    let (listener, addr) = bind(&cfg).await.expect("bind");
    tokio::spawn(async move { serve(listener, cfg, world).await });
    addr
}

async fn connect(addr: std::net::SocketAddr) -> Framed<TcpStream, ClientSessionCodec> {
    let tcp = TcpStream::connect(addr).await.expect("connect");
    Framed::new(tcp, ClientSessionCodec::new())
}

/// Send the handshake only.
async fn send_handshake(client: &mut Framed<TcpStream, ClientSessionCodec>, username: &str) {
    client
        .send(ClientMessage::Handshake(Handshake {
            protocol_version: PROTOCOL_VERSION,
            client_build: "e2e-test".to_string(),
            username: username.to_string(),
        }))
        .await
        .expect("send handshake");
}

/// Full auth: send the handshake, consume the result, return the player id.
async fn handshake_ok(client: &mut Framed<TcpStream, ClientSessionCodec>, username: &str) -> u64 {
    send_handshake(client, username).await;
    match client
        .next()
        .await
        .expect("server replies")
        .expect("no codec error")
    {
        ServerMessage::HandshakeResult {
            accepted,
            player_id,
            ..
        } => {
            assert!(accepted, "handshake must be accepted");
            player_id
        }
        other => panic!("unexpected message during auth: {other:?}"),
    }
}

/// Consume the post-auth world stream (`PlayerList` skipped), returning
/// `(world_info, chunks)`.
async fn read_world(
    client: &mut Framed<TcpStream, ClientSessionCodec>,
) -> (WorldInfo, Vec<ChunkData>) {
    let mut info: Option<WorldInfo> = None;
    let mut chunks = Vec::new();
    loop {
        let msg = client
            .next()
            .await
            .expect("server replies")
            .expect("no codec error");
        match msg {
            ServerMessage::HandshakeResult {
                accepted,
                rejection,
                ..
            } => {
                assert!(accepted, "auth must succeed: {rejection:?}");
            }
            // The roster snapshot precedes the world; skipped here (the
            // presence tests assert on it explicitly).
            ServerMessage::PlayerList { .. } => {}
            ServerMessage::WorldInfo(info_msg) => {
                assert!(chunks.is_empty(), "WorldInfo must come first");
                info = Some(info_msg);
            }
            ServerMessage::ChunkData(chunk) => {
                let expected = info.as_ref().expect("chunk before WorldInfo").chunks;
                chunks.push(chunk);
                assert!((chunks.len() as u32) <= expected, "too many chunks");
                if chunks.len() as u32 == expected {
                    return (info.expect("info set"), chunks);
                }
            }
            other => panic!("unexpected message during world stream: {other:?}"),
        }
    }
}

/// Drain presence broadcasts (join echoes, moves, chat) until the stream is
/// quiet for a moment. Messages the caller cares about are returned so they
/// aren't lost.
async fn skip_presence(
    client: &mut Framed<TcpStream, ClientSessionCodec>,
) -> Option<ServerMessage> {
    loop {
        match tokio::time::timeout(Duration::from_millis(200), client.next()).await {
            Err(_) => return None,   // quiet — nothing pending
            Ok(None) => return None, // stream closed
            Ok(Some(Err(err))) => panic!("codec error while draining: {err}"),
            Ok(Some(Ok(m))) => match m {
                ServerMessage::PlayerJoin { .. }
                | ServerMessage::PlayerLeave { .. }
                | ServerMessage::PlayerMove { .. }
                | ServerMessage::Chat { .. } => continue,
                other => return Some(other),
            },
        }
    }
}

// --- Phase 1: session spine -------------------------------------------------

#[tokio::test]
async fn handshake_is_accepted_and_motd_returned() {
    let addr = spawn_server().await;
    let mut client = connect(addr).await;
    send_handshake(&mut client, "Tester").await;

    let reply = client
        .next()
        .await
        .expect("server replies")
        .expect("no codec error");
    match reply {
        ServerMessage::HandshakeResult {
            accepted,
            player_id,
            server_name,
            motd,
            rejection,
            ..
        } => {
            assert!(accepted, "valid handshake must be accepted: {rejection:?}");
            assert!(player_id > 0, "accepted clients get a player id");
            assert_eq!(server_name, "MELON-test");
            assert_eq!(motd, "hello from the test server");
            assert!(rejection.is_none());
        }
        other => panic!("unexpected message: {other:?}"),
    }
}

#[tokio::test]
async fn ping_pong_roundtrip_preserves_payload() {
    let addr = spawn_server().await;
    let mut client = connect(addr).await;
    handshake_ok(&mut client, "Pinger").await;
    read_world(&mut client).await;
    skip_presence(&mut client).await;

    for payload in [0u64, 1, 0xDEAD_BEEF, u64::MAX] {
        let ts = payload.wrapping_add(1000);
        client
            .send(ClientMessage::Ping {
                payload,
                client_timestamp_ms: ts,
            })
            .await
            .expect("send ping");
        match client.next().await.expect("pong").expect("no codec error") {
            ServerMessage::Pong {
                payload: p,
                client_timestamp_ms: echoed,
                server_timestamp_ms: now,
            } => {
                assert_eq!(p, payload);
                assert_eq!(echoed, ts);
                assert!(now > 0, "server timestamp must be set");
            }
            other => panic!("unexpected message: {other:?}"),
        }
    }
}

#[tokio::test]
async fn version_mismatch_is_rejected_with_reason() {
    let addr = spawn_server().await;
    let mut client = connect(addr).await;
    client
        .send(ClientMessage::Handshake(Handshake {
            protocol_version: PROTOCOL_VERSION + 7,
            client_build: "future-client".to_string(),
            username: "TimeTraveler".to_string(),
        }))
        .await
        .expect("send handshake");

    match client
        .next()
        .await
        .expect("rejection")
        .expect("no codec error")
    {
        ServerMessage::HandshakeResult {
            accepted,
            protocol_version,
            rejection,
            ..
        } => {
            assert!(!accepted);
            assert_eq!(protocol_version, PROTOCOL_VERSION);
            assert_eq!(
                rejection,
                Some(HandshakeRejectReason::ProtocolVersionMismatch {
                    expected: PROTOCOL_VERSION,
                    got: PROTOCOL_VERSION + 7,
                })
            );
        }
        other => panic!("unexpected message: {other:?}"),
    }

    // The server must close the stream after a rejection.
    assert!(
        client.next().await.is_none(),
        "connection must close after rejection"
    );
}

#[tokio::test]
async fn invalid_username_is_rejected() {
    let addr = spawn_server().await;
    let mut client = connect(addr).await;
    send_handshake(&mut client, "bad name!").await;

    match client
        .next()
        .await
        .expect("rejection")
        .expect("no codec error")
    {
        ServerMessage::HandshakeResult {
            accepted,
            rejection,
            ..
        } => {
            assert!(!accepted);
            let Some(HandshakeRejectReason::InvalidUsername { reason }) = rejection else {
                panic!("expected InvalidUsername, got {rejection:?}");
            };
            assert!(
                reason.contains("letters"),
                "reason explains the rule: {reason}"
            );
        }
        other => panic!("unexpected message: {other:?}"),
    }
}

#[tokio::test]
async fn message_before_handshake_is_rejected() {
    let addr = spawn_server().await;
    let mut client = connect(addr).await;

    client
        .send(ClientMessage::Ping {
            payload: 1,
            client_timestamp_ms: 1,
        })
        .await
        .expect("send ping before handshake");

    // The server must drop the connection without an auth result.
    assert!(
        client.next().await.is_none(),
        "no session may start without a handshake"
    );
}

// --- Phase 3: world streaming + block edits --------------------------------

#[tokio::test]
async fn world_streams_after_auth() {
    let addr = spawn_server().await;
    let mut client = connect(addr).await;
    handshake_ok(&mut client, "Builder").await;

    let (info, chunks) = read_world(&mut client).await;
    assert_eq!(info.chunks, 4, "2×2 world = 4 chunks");
    assert_eq!(chunks.len(), 4);
    assert!(info.spawn[1] > 6.0, "spawn floats above the grass");
    for chunk in &chunks {
        assert_ne!(
            chunk.section_mask, 0,
            "superflat chunks have a surface section"
        );
        // One non-empty section = 4096 LE u16s = 8192 bytes.
        assert_eq!(
            chunk.sections.len(),
            chunk.section_mask.count_ones() as usize * 8192
        );
    }
}

#[tokio::test]
async fn block_edit_broadcasts_to_every_client() {
    let addr = spawn_server().await;
    let mut a = connect(addr).await;
    handshake_ok(&mut a, "Alice").await;
    read_world(&mut a).await;

    let mut b = connect(addr).await;
    handshake_ok(&mut b, "Bob").await;
    read_world(&mut b).await;

    // Drain the join echoes both clients received for each other.
    skip_presence(&mut a).await;
    skip_presence(&mut b).await;

    // Alice edits a block — both Alice and Bob must receive the update.
    a.send(ClientMessage::SetBlock {
        x: 2,
        y: 6,
        z: 3,
        block: block::WOOL_RED,
    })
    .await
    .expect("send edit");

    for (who, client) in [("Alice", &mut a), ("Bob", &mut b)] {
        match client
            .next()
            .await
            .expect("block update")
            .expect("no codec error")
        {
            ServerMessage::BlockUpdate { x, y, z, block } => {
                assert_eq!(
                    (x, y, z, block),
                    (2, 6, 3, block::WOOL_RED),
                    "{who} sees it"
                );
            }
            other => panic!("{who} got unexpected message: {other:?}"),
        }
    }
}

#[tokio::test]
async fn invalid_edit_is_silently_rejected_and_session_survives() {
    let addr = spawn_server().await;
    let mut a = connect(addr).await;
    handshake_ok(&mut a, "Griefer").await;
    read_world(&mut a).await;
    skip_presence(&mut a).await;

    // Out of bounds in y.
    a.send(ClientMessage::SetBlock {
        x: 1,
        y: 999,
        z: 1,
        block: block::STONE,
    })
    .await
    .expect("send invalid edit");
    // Out of bounds in x/z (beyond the 2×2-chunk world).
    a.send(ClientMessage::SetBlock {
        x: 500,
        y: 6,
        z: 500,
        block: block::STONE,
    })
    .await
    .expect("send invalid edit");

    // Neither produced a broadcast: a short wait proves silence…
    let noise = tokio::time::timeout(Duration::from_millis(250), a.next()).await;
    assert!(noise.is_err(), "invalid edits must not broadcast");

    // …and the session is still healthy: a valid edit goes through.
    a.send(ClientMessage::SetBlock {
        x: 1,
        y: 6,
        z: 1,
        block: block::WOOL_BLUE,
    })
    .await
    .expect("session alive");
    match a
        .next()
        .await
        .expect("update after rejections")
        .expect("no codec error")
    {
        ServerMessage::BlockUpdate { x, y, z, block } => {
            assert_eq!((x, y, z, block), (1, 6, 1, block::WOOL_BLUE));
        }
        other => panic!("unexpected message: {other:?}"),
    }
}

// --- Phase 5: held-item sync -----------------------------------------------

#[tokio::test]
async fn held_item_relay_stamps_sender_id() {
    let addr = spawn_server().await;
    let mut a = connect(addr).await;
    let a_id = handshake_ok(&mut a, "Alice").await;
    read_world(&mut a).await;
    skip_presence(&mut a).await;

    a.send(ClientMessage::HeldItem { slot: 4 })
        .await
        .expect("send held");
    match a.next().await.expect("held relay").expect("no codec error") {
        ServerMessage::PlayerHeld { id, slot } => {
            assert_eq!(id, a_id);
            assert_eq!(slot, 4);
        }
        other => panic!("unexpected message: {other:?}"),
    }

    // Invalid slots are dropped, not relayed.
    a.send(ClientMessage::HeldItem { slot: 42 })
        .await
        .expect("send bad slot");
    let noise = tokio::time::timeout(Duration::from_millis(250), a.next()).await;
    assert!(noise.is_err(), "invalid slot must not relay");
}

#[tokio::test]
async fn presence_lifecycle_join_roster_leave() {
    let addr = spawn_server().await;

    // Alice joins first — the roster she sees is empty.
    let mut a = connect(addr).await;
    let a_id = handshake_ok(&mut a, "Alice").await;
    read_world(&mut a).await;
    skip_presence(&mut a).await;

    // Bob joins — his roster contains Alice, and he learns his own id.
    let mut b = connect(addr).await;
    let b_id = handshake_ok(&mut b, "Bob").await;
    assert_ne!(a_id, b_id, "player ids must be unique");

    // B's world stream is preceded by the roster with Alice in it.
    let roster = loop {
        match b.next().await.expect("roster").expect("no codec error") {
            ServerMessage::PlayerList { players } => break players,
            ServerMessage::HandshakeResult { .. } => continue,
            other => panic!("unexpected message before roster: {other:?}"),
        }
    };
    assert_eq!(
        roster,
        vec![PlayerEntry {
            id: a_id,
            name: "Alice".to_string()
        }]
    );

    // B still has its own world stream + join echo queued — drain them.
    read_world(&mut b).await;
    skip_presence(&mut b).await;

    // Alice receives Bob's join broadcast.
    match a
        .next()
        .await
        .expect("join broadcast")
        .expect("no codec error")
    {
        ServerMessage::PlayerJoin { id, name } => {
            assert_eq!((id, name.as_str()), (b_id, "Bob"));
        }
        other => panic!("unexpected message: {other:?}"),
    }

    // Alice leaves → Bob is told.
    drop(a);
    match b
        .next()
        .await
        .expect("leave broadcast")
        .expect("no codec error")
    {
        ServerMessage::PlayerLeave { id } => assert_eq!(id, a_id),
        other => panic!("unexpected message: {other:?}"),
    }
}

#[tokio::test]
async fn moves_and_chat_relay_to_other_players() {
    let addr = spawn_server().await;

    let mut a = connect(addr).await;
    let a_id = handshake_ok(&mut a, "Alice").await;
    read_world(&mut a).await;

    let mut b = connect(addr).await;
    handshake_ok(&mut b, "Bob").await;
    read_world(&mut b).await;
    skip_presence(&mut a).await;
    skip_presence(&mut b).await;

    // Alice reports a position — Bob receives it (Alice's own echo is
    // filtered by the client, not the server).
    a.send(ClientMessage::Move(melon_proto::PlayerMove {
        x: 10.0,
        y: 8.0,
        z: 20.0,
        yaw: 1.0,
        pitch: 0.25,
    }))
    .await
    .expect("send move");

    match b.next().await.expect("move relay").expect("no codec error") {
        ServerMessage::PlayerMove {
            id,
            pos,
            yaw,
            pitch,
        } => {
            assert_eq!(id, a_id);
            assert_eq!(pos, [10.0, 8.0, 20.0]);
            assert_eq!((yaw, pitch), (1.0, 0.25));
        }
        other => panic!("unexpected message: {other:?}"),
    }

    // Alice chats — Bob gets it stamped with her identity.
    a.send(ClientMessage::Chat {
        text: "gg".to_string(),
    })
    .await
    .expect("send chat");
    match b.next().await.expect("chat relay").expect("no codec error") {
        ServerMessage::Chat { id, name, text } => {
            assert_eq!(id, a_id);
            assert_eq!(name, "Alice");
            assert_eq!(text, "gg");
        }
        other => panic!("unexpected message: {other:?}"),
    }
}

#[tokio::test]
async fn chat_with_control_characters_is_rejected() {
    let addr = spawn_server().await;
    let mut a = connect(addr).await;
    handshake_ok(&mut a, "Chatter").await;
    read_world(&mut a).await;
    skip_presence(&mut a).await;

    a.send(ClientMessage::Chat {
        text: "bad\u{0007}noise".to_string(),
    })
    .await
    .expect("send chat");

    // No broadcast within the window proves the rejection…
    let noise = tokio::time::timeout(Duration::from_millis(250), a.next()).await;
    assert!(noise.is_err(), "invalid chat must not broadcast");
}
