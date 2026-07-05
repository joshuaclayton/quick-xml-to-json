use serde::Serialize;
use std::io::{self, Write};

#[derive(thiserror::Error, Debug)]
pub enum BufferError {
    #[error("buffer write error: {0}")]
    OnBufferWrite(#[from] io::Error),

    #[error("JSON string write error: {0}")]
    OnSerdeJson(#[from] serde_json::Error),
}

/// Write a JSON-escaped string to `buffer`, using a fast path when possible.
///
/// This avoids allocation on the slow path by streaming via `serde_json::Serializer`.
pub(super) fn write_json_string<W: Write>(s: &str, buffer: &mut W) -> Result<(), BufferError> {
    if needs_escaping_fast(s) {
        // Slow path: stream quoted+escaped directly into `buffer` (no intermediate String).
        let mut ser = serde_json::Serializer::new(buffer);
        s.serialize(&mut ser)?;
    } else {
        buffer.write_all(b"\"")?;
        buffer.write_all(s.as_bytes())?;
        buffer.write_all(b"\"")?;
    }

    Ok(())
}

fn needs_escaping_fast(s: &str) -> bool {
    for &b in s.as_bytes() {
        if b == b'"' || b == b'\\' || b < 0x20 {
            return true;
        }
        // Non-ASCII UTF-8 bytes are valid in JSON strings without escaping
    }
    false
}
