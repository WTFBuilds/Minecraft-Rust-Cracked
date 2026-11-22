//! Frame codec: VarInt-prefixed, bincode-encoded messages over any byte
//! transport, via `tokio_util::codec`.
//!
//! Two perspective codecs share the framing rules:
//! - [`ServerSessionCodec`] — **server side** of a connection: reads
//!   [`ClientMessage`]s, writes [`ServerMessage`]s.
//! - [`ClientSessionCodec`] — **client side**: the mirror image.
//!
//! One connection = one codec instance = no cross-direction confusion.

use bytes::{BufMut, BytesMut};
use serde::de::DeserializeOwned;
use serde::Serialize;
use tokio_util::codec::{Decoder, Encoder};

use crate::error::ProtoError;
use crate::message::{ClientMessage, ServerMessage, MAX_FRAME_SIZE};
use crate::varint::read_varint;

/// Decode one frame from `src`, buffering partial deliveries.
fn decode_frame<M: DeserializeOwned>(src: &mut BytesMut) -> Result<Option<M>, ProtoError> {
    if src.is_empty() {
        return Ok(None);
    }

    // Peek the VarInt length without consuming — if the payload hasn't
    // arrived yet we must keep the header bytes in the buffer.
    let (payload_len, header_len) = match read_varint(src) {
        Ok(pair) => pair,
        Err(crate::varint::VarIntError::TooShort) => return Ok(None),
        Err(err) => return Err(err.into()),
    };

    if payload_len > MAX_FRAME_SIZE {
        return Err(ProtoError::FrameTooLarge(payload_len));
    }

    let available = src.len() - header_len;
    if available < payload_len as usize {
        src.reserve(header_len + payload_len as usize - available);
        return Ok(None);
    }

    let frame = src.split_to(header_len + payload_len as usize);
    let (msg, _used): (M, usize) =
        bincode::serde::decode_from_slice(&frame[header_len..], bincode::config::standard())?;
    Ok(Some(msg))
}

/// Encode one frame into `dst`.
fn encode_frame<M: Serialize>(item: &M, dst: &mut BytesMut) -> Result<(), ProtoError> {
    let payload = bincode::serde::encode_to_vec(item, bincode::config::standard())?;
    if payload.len() as u32 > MAX_FRAME_SIZE {
        return Err(ProtoError::FrameTooLarge(payload.len() as u32));
    }
    let mut value = payload.len() as u32;
    loop {
        if value & !0x7F == 0 {
            dst.put_u8(value as u8);
            break;
        }
        dst.put_u8(((value & 0x7F) | 0x80) as u8);
        value >>= 7;
    }
    dst.extend_from_slice(&payload);
    Ok(())
}

/// Server side of a session: decodes [`ClientMessage`], encodes [`ServerMessage`].
#[derive(Debug, Clone, Default)]
pub struct ServerSessionCodec;

impl ServerSessionCodec {
    pub fn new() -> Self {
        Self
    }
}

impl Decoder for ServerSessionCodec {
    type Item = ClientMessage;
    type Error = ProtoError;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        decode_frame(src)
    }
}

impl Encoder<ServerMessage> for ServerSessionCodec {
    type Error = ProtoError;

    fn encode(&mut self, item: ServerMessage, dst: &mut BytesMut) -> Result<(), Self::Error> {
        encode_frame(&item, dst)
    }
}

/// Client side of a session: decodes [`ServerMessage`], encodes [`ClientMessage`].
#[derive(Debug, Clone, Default)]
pub struct ClientSessionCodec;

impl ClientSessionCodec {
    pub fn new() -> Self {
        Self
    }
}

impl Decoder for ClientSessionCodec {
    type Item = ServerMessage;
    type Error = ProtoError;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        decode_frame(src)
    }
}

impl Encoder<ClientMessage> for ClientSessionCodec {
    type Error = ProtoError;

    fn encode(&mut self, item: ClientMessage, dst: &mut BytesMut) -> Result<(), Self::Error> {
        encode_frame(&item, dst)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::message::{Handshake, PROTOCOL_VERSION};

    fn client_handshake() -> ClientMessage {
        ClientMessage::Handshake(Handshake {
            protocol_version: PROTOCOL_VERSION,
            client_build: "test".to_string(),
            username: "Tester".to_string(),
        })
    }

    fn server_pong() -> ServerMessage {
        ServerMessage::Pong {
            payload: 7,
            client_timestamp_ms: 100,
            server_timestamp_ms: 105,
        }
    }

    #[test]
    fn frame_survives_chunked_delivery() {
        let wire = {
            let mut buf = BytesMut::new();
            ClientSessionCodec
                .encode(client_handshake(), &mut buf)
                .expect("encode");
            buf.freeze()
        };

        let mut codec = ServerSessionCodec;
        let mut partial = BytesMut::new();
        let chunks: Vec<&[u8]> = wire.chunks(3).collect();

        for (i, chunk) in chunks.iter().enumerate() {
            partial.extend_from_slice(chunk);
            let decoded = codec.decode(&mut partial).expect("decode must not error");
            if i == chunks.len() - 1 {
                assert_eq!(decoded, Some(client_handshake()));
            } else {
                assert_eq!(decoded, None, "partial delivery must not decode");
            }
        }
        assert!(partial.is_empty(), "frame bytes must be consumed exactly");
    }

    #[test]
    fn two_frames_decode_in_sequence() {
        let mut buf = BytesMut::new();
        ClientSessionCodec
            .encode(client_handshake(), &mut buf)
            .unwrap();
        ClientSessionCodec
            .encode(
                ClientMessage::Ping {
                    payload: 7,
                    client_timestamp_ms: 100,
                },
                &mut buf,
            )
            .unwrap();

        let mut codec = ServerSessionCodec;
        assert_eq!(codec.decode(&mut buf).unwrap(), Some(client_handshake()));
        assert_eq!(
            codec.decode(&mut buf).unwrap(),
            Some(ClientMessage::Ping {
                payload: 7,
                client_timestamp_ms: 100
            })
        );
        assert_eq!(codec.decode(&mut buf).unwrap(), None);
    }

    #[test]
    fn server_side_writes_and_reads_its_own_direction() {
        // The server encodes ServerMessage and decodes ClientMessage — the
        // perspective codecs must agree on framing with each other.
        let mut server_wire = BytesMut::new();
        ServerSessionCodec
            .encode(server_pong(), &mut server_wire)
            .unwrap();

        let mut client_codec = ClientSessionCodec;
        let decoded = client_codec
            .decode(&mut server_wire)
            .expect("decode")
            .expect("full frame");
        assert_eq!(decoded, server_pong());
        assert!(server_wire.is_empty());
    }

    #[test]
    fn oversized_length_is_rejected() {
        let mut codec = ServerSessionCodec;
        let mut buf = BytesMut::new();
        // Craft a length prefix way beyond MAX_FRAME_SIZE (5-byte VarInt).
        buf.extend_from_slice(&[0xFF, 0xFF, 0xFF, 0xFF, 0x0F]); // VarInt(u32::MAX)
        buf.extend_from_slice(&[0u8; 8]);
        match codec.decode(&mut buf) {
            Err(ProtoError::FrameTooLarge(len)) => assert_eq!(len, u32::MAX),
            other => panic!("expected FrameTooLarge, got {other:?}"),
        }
    }

    #[test]
    fn garbage_payload_errors_instead_of_panicking() {
        let mut codec = ServerSessionCodec;
        let mut buf = BytesMut::new();
        buf.extend_from_slice(&[0x05, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF]); // claims 5 bytes of junk
        buf.extend_from_slice(&[0u8; 5]);
        assert!(
            codec.decode(&mut buf).is_err(),
            "invalid payload must error"
        );
    }
}
