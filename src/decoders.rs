use crate::XmlToJsonError;
use quick_xml::escape::resolve_predefined_entity;
use quick_xml::{Reader, events::BytesText};
use std::borrow::Cow;
use std::io::BufRead;

pub fn decode_bytes<'a, R: BufRead>(
    reader: &Reader<R>,
    bytes: &'a [u8],
) -> Result<Cow<'a, str>, quick_xml::Error> {
    Ok(reader.decoder().decode(bytes)?)
}

pub fn decode_text<'a, R: BufRead>(
    reader: &Reader<R>,
    text: &'a BytesText<'a>,
) -> Result<Cow<'a, str>, quick_xml::Error> {
    Ok(reader.decoder().decode(text)?)
}

/// Resolve XML references in `raw` without failing on entities we cannot know about.
///
/// Predefined entities (`&amp;` etc.) and character references (`&#65;`, `&#x41;`) are
/// resolved. Named entities we cannot resolve (e.g. DTD-defined ones like `&uuml;`) and
/// ampersands that do not form a reference pass through verbatim rather than erroring or
/// being dropped. Malformed character references error, since no DTD can make them valid.
pub fn unescape_lenient(raw: &str) -> Result<Cow<'_, str>, XmlToJsonError> {
    if !raw.contains('&') {
        return Ok(Cow::Borrowed(raw));
    }

    let mut out = String::with_capacity(raw.len());
    let mut parts = raw.split('&');

    if let Some(prefix) = parts.next() {
        out.push_str(prefix);
    }

    for part in parts {
        if let Some((name, rest)) = part.split_once(';') {
            push_resolved_ref(name, &mut out)?;
            out.push_str(rest);
        } else {
            // No terminating `;`, so this is a bare ampersand; keep it verbatim
            out.push('&');
            out.push_str(part);
        }
    }

    Ok(Cow::Owned(out))
}

fn push_resolved_ref(name: &str, out: &mut String) -> Result<(), XmlToJsonError> {
    if let Some(digits) = name.strip_prefix('#') {
        let ch = parse_char_ref(digits)
            .ok_or_else(|| XmlToJsonError::InvalidCharRef(name.to_string()))?;
        out.push(ch);
    } else if let Some(resolved) = resolve_predefined_entity(name) {
        out.push_str(resolved);
    } else {
        out.push('&');
        out.push_str(name);
        out.push(';');
    }

    Ok(())
}

fn parse_char_ref(digits: &str) -> Option<char> {
    let code = if let Some(hex) = digits.strip_prefix('x') {
        parse_u32(hex, 16)?
    } else {
        parse_u32(digits, 10)?
    };

    if code == 0 {
        return None;
    }

    char::from_u32(code)
}

// `u32` parsing accepts a leading `+`, which the XML grammar does not; reject signs
// outright so character references behave identically in attributes and text (where
// quick-xml's parser does the same).
fn parse_u32(src: &str, radix: u32) -> Option<u32> {
    if matches!(src.as_bytes().first(), Some(b'+' | b'-')) {
        return None;
    }

    u32::from_str_radix(src, radix).ok()
}

#[cfg(test)]
mod tests {
    use super::unescape_lenient;
    use std::borrow::Cow;

    #[test]
    fn borrows_when_no_ampersand() {
        let result = unescape_lenient("plain value").unwrap();
        assert!(
            matches!(result, Cow::Borrowed("plain value")),
            "expected borrowed passthrough, got {result:?}"
        );
    }

    #[test]
    fn resolves_predefined_and_char_refs() {
        assert_eq!("a&b", unescape_lenient("a&amp;b").unwrap());
        assert_eq!("5 < 6 > 4", unescape_lenient("5 &lt; 6 &gt; 4").unwrap());
        assert_eq!(
            "\"q\" 's'",
            unescape_lenient("&quot;q&quot; &apos;s&apos;").unwrap()
        );
        assert_eq!("ABC", unescape_lenient("A&#66;C").unwrap());
        assert_eq!("AbC", unescape_lenient("A&#x62;C").unwrap());
    }

    #[test]
    fn passes_through_unknown_entities_and_bare_ampersands() {
        assert_eq!("x &uuml; y", unescape_lenient("x &uuml; y").unwrap());
        assert_eq!("q?x=1&y=2", unescape_lenient("q?x=1&y=2").unwrap());
        assert_eq!("a & b; c", unescape_lenient("a & b; c").unwrap());
        assert_eq!("&;x", unescape_lenient("&;x").unwrap());
        assert_eq!("tail&", unescape_lenient("tail&").unwrap());
    }

    #[test]
    fn errors_on_malformed_char_refs() {
        assert!(
            unescape_lenient("&#xZZ;").is_err(),
            "hex garbage should error"
        );
        assert!(
            unescape_lenient("&#;").is_err(),
            "empty char ref should error"
        );
        assert!(
            unescape_lenient("&#0;").is_err(),
            "NUL is never a valid XML char"
        );
        assert!(
            unescape_lenient("&#xD800;").is_err(),
            "surrogates are not chars"
        );
        assert!(
            unescape_lenient("&#+65;").is_err(),
            "signs are not part of the XML char ref grammar"
        );
        assert!(
            unescape_lenient("&#x+41;").is_err(),
            "signs are not part of the XML char ref grammar"
        );
    }
}
