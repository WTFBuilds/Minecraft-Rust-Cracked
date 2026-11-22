//! Unsigned LEB128 VarInt codec — compact integer encoding for frame lengths
//! and counts. Maximum 5 bytes for a `u32`.

use thiserror::Error;

const MAX_VARINT_BYTES: usize = 5;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum VarIntError {
    #[error("varint truncated in the middle")]
    TooShort,
    #[error("varint longer than {MAX_VARINT_BYTES} bytes")]
    TooLong,
}

/// Encode a `u32` as an unsigned LEB128 VarInt into `buf` (1–5 bytes).
pub fn write_varint(buf: &mut Vec<u8>, value: u32) {
    let mut value = value;
    loop {
        if value & !0x7F == 0 {
            buf.push(value as u8);
            return;
        }
        buf.push(((value & 0x7F) | 0x80) as u8);
        value >>= 7;
    }
}

/// Encode a `u32` into a fixed-size buffer; returns the written slice.
pub fn varint_bytes(value: u32) -> [u8; 5] {
    let mut tmp = Vec::with_capacity(5);
    write_varint(&mut tmp, value);
    let mut out = [0u8; 5];
    out[..tmp.len()].copy_from_slice(&tmp);
    out
}

/// Decode a VarInt from the front of `buf` **without consuming it**.
/// Returns `(value, bytes_consumed)`.
pub fn read_varint(buf: &[u8]) -> Result<(u32, usize), VarIntError> {
    let mut value: u32 = 0;
    for (i, &byte) in buf.iter().take(MAX_VARINT_BYTES).enumerate() {
        value |= u32::from(byte & 0x7F) << (i * 7);
        if byte & 0x80 == 0 {
            return Ok((value, i + 1));
        }
    }
    if buf.len() >= MAX_VARINT_BYTES {
        Err(VarIntError::TooLong)
    } else {
        Err(VarIntError::TooShort)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn roundtrip(value: u32) {
        let mut buf = Vec::new();
        write_varint(&mut buf, value);
        let (decoded, used) = read_varint(&buf).expect("decode");
        assert_eq!(decoded, value, "roundtrip failed for {value}");
        assert_eq!(
            used,
            buf.len(),
            "varint must be minimally encoded for {value}"
        );
    }

    #[test]
    fn boundary_values() {
        roundtrip(0);
        roundtrip(1);
        roundtrip(0x7F); // 1-byte max
        roundtrip(0x80); // first 2-byte value
        roundtrip(0x3FFF); // 2-byte max
        roundtrip(0x200000); // 3-byte range
        roundtrip(0x08000000); // 4-byte range
        roundtrip(u32::MAX); // 5-byte max
    }

    #[test]
    fn minimal_encoding() {
        let mut buf = Vec::new();
        write_varint(&mut buf, 0x7F);
        assert_eq!(buf.len(), 1);
        buf.clear();
        write_varint(&mut buf, 0x80);
        assert_eq!(buf.len(), 2);
    }

    #[test]
    fn truncated_varint_is_too_short() {
        let mut buf = Vec::new();
        write_varint(&mut buf, 0x200000); // 3 bytes
        assert_eq!(read_varint(&buf[..1]), Err(VarIntError::TooShort));
        assert_eq!(read_varint(&buf[..2]), Err(VarIntError::TooShort));
    }

    #[test]
    fn overlong_varint_is_too_long() {
        // 5 continuation bytes = 6 bytes total: illegal.
        let buf = [0x80, 0x80, 0x80, 0x80, 0x80, 0x00];
        assert_eq!(read_varint(&buf), Err(VarIntError::TooLong));
    }

    #[test]
    fn varint_bytes_matches_vec_version() {
        for value in [0u32, 1, 127, 128, 12345, u32::MAX] {
            let fixed = varint_bytes(value);
            let mut dynamic = Vec::new();
            write_varint(&mut dynamic, value);
            assert_eq!(&fixed[..dynamic.len()], &dynamic[..]);
        }
    }
}
