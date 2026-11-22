//! End-to-end protocol tests: a real TCP round trip through the server,
//! plus a headless client session — the Phase 1 acceptance criteria.

use futures::{SinkExt, StreamExt};
use melon_proto::{
    ClientMessage, ClientSessionCodec, Handshake, HandshakeRejectReason, ServerMessage,
    PROTOCOL_VERSION,
};
use melon_server::{bind, serve, ServerConfig};
use tokio::net::TcpStream;
use tokio_util::codec::Framed;

fn test_config() -> ServerConfig {
    ServerConfig {
        bind_addr: "127.0.0.1:0".parse().expect("test addr is valid"),
        server_name: "MELON-test".to_string(),
        motd: "hello from the test server".to_string(),
    }
}

async fn spawn_server() -> std::net::SocketAddr {
    let cfg = test_config();
    let (listener, addr) = bind(&cfg).await.expect("bind");
    tokio::spawn(async move { serve(listener, cfg).await });
    addr
}

async fn connect(addr: std::net::SocketAddr) -> Framed<TcpStream, ClientSessionCodec> {
    let tcp = TcpStream::connect(addr).await.expect("connect");
    Framed::new(tcp, ClientSessionCodec::new())
}

#[tokio::test]
async fn handshake_is_accepted_and_motd_returned() {
    let addr = spawn_server().await;
    let mut client = connect(addr).await;

    client
        .send(ClientMessage::Handshake(Handshake {
            protocol_version: PROTOCOL_VERSION,
            client_build: "e2e-test".to_string(),
            username: "Tester".to_string(),
        }))
        .await
        .expect("send handshake");

    let reply = client
        .next()
        .await
        .expect("server replies")
        .expect("no codec error");
    match reply {
        ServerMessage::HandshakeResult {
            accepted,
            server_name,
            motd,
            rejection,
            ..
        } => {
            assert!(accepted, "valid handshake must be accepted: {rejection:?}");
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

    client
        .send(ClientMessage::Handshake(Handshake {
            protocol_version: PROTOCOL_VERSION,
            client_build: "e2e-test".to_string(),
            username: "Pinger".to_string(),
        }))
        .await
        .expect("send handshake");
    client
        .next()
        .await
        .expect("handshake result")
        .expect("no codec error");

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

    client
        .send(ClientMessage::Handshake(Handshake {
            protocol_version: PROTOCOL_VERSION,
            client_build: "e2e-test".to_string(),
            username: "bad name!".to_string(),
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
