//! Protocol error surface.

use thiserror::Error;

use crate::message::MAX_FRAME_SIZE;
use crate::varint::VarIntError;

#[derive(Debug, Error)]
pub enum ProtoError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    #[error("encode error: {0}")]
    Encode(#[from] bincode::error::EncodeError),

    #[error("decode error: {0}")]
    Decode(#[from] bincode::error::DecodeError),

    #[error("malformed varint: {0}")]
    VarInt(#[from] VarIntError),

    #[error("frame of {0} bytes exceeds the {MAX_FRAME_SIZE} byte limit")]
    FrameTooLarge(u32),

    #[error("stream closed mid-frame")]
    UnexpectedEof,
}
