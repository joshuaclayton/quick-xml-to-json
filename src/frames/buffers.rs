use std::io::{self, Write};

/// Write `s` as a quoted JSON string, escaping in a single pass.
///
/// Clean runs are written directly from the input; only bytes requiring an escape
/// (`"`, `\`, and control characters below 0x20) interrupt the run. Escape sequences
/// match `serde_json`'s output byte-for-byte (verified in tests below).
pub(super) fn write_json_string<W: Write>(s: &str, buffer: &mut W) -> io::Result<()> {
    buffer.write_all(b"\"")?;

    let bytes = s.as_bytes();
    let mut run_start = 0;

    for (i, &b) in bytes.iter().enumerate() {
        if !needs_escape(b) {
            continue;
        }
        if run_start < i {
            buffer.write_all(bytes.get(run_start..i).unwrap_or_default())?;
        }
        write_escape(b, buffer)?;
        run_start = i + 1;
    }

    if let Some(tail) = bytes.get(run_start..) {
        buffer.write_all(tail)?;
    }
    buffer.write_all(b"\"")?;

    Ok(())
}

const fn needs_escape(b: u8) -> bool {
    matches!(b, b'"' | b'\\') || b < 0x20
}

fn write_escape<W: Write>(b: u8, buffer: &mut W) -> io::Result<()> {
    match b {
        b'"' => buffer.write_all(b"\\\""),
        b'\\' => buffer.write_all(b"\\\\"),
        b'\n' => buffer.write_all(b"\\n"),
        b'\r' => buffer.write_all(b"\\r"),
        b'\t' => buffer.write_all(b"\\t"),
        0x08 => buffer.write_all(b"\\b"),
        0x0C => buffer.write_all(b"\\f"),
        _ => {
            let out = [
                b'\\',
                b'u',
                b'0',
                b'0',
                hex_digit(b >> 4),
                hex_digit(b & 0xF),
            ];
            buffer.write_all(&out)
        }
    }
}

const fn hex_digit(nibble: u8) -> u8 {
    let n = nibble & 0xF;
    if n < 10 { b'0' + n } else { b'a' + (n - 10) }
}

#[cfg(test)]
mod tests {
    use super::write_json_string;

    fn escaped(s: &str) -> String {
        let mut out = Vec::new();
        write_json_string(s, &mut out).unwrap();
        String::from_utf8(out).unwrap()
    }

    #[test]
    fn matches_serde_json_byte_for_byte() {
        let cases = [
            "",
            "plain text",
            "with \"quotes\" inside",
            "back\\slash",
            "line1\nline2\r\ttabbed",
            "\u{8}backspace and \u{c}formfeed",
            "\u{0}\u{1}\u{2}\u{3}\u{1f}",
            "unicode: über ünïcode 日本語 👍",
            "mixed \"q\" and \u{1} and ü",
            "trailing escape\n",
            "\nleading escape",
        ];

        for case in cases {
            let expected = serde_json::to_string(case).unwrap();
            assert_eq!(
                expected,
                escaped(case),
                "escaping diverges from serde_json for {case:?}"
            );
        }
    }
}
