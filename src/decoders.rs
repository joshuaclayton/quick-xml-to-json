use quick_xml::{Reader, events::BytesText};
use std::borrow::Cow;
use std::io::BufRead;

pub(crate) fn decode_bytes<'a, R: BufRead>(
    reader: &Reader<R>,
    bytes: &'a [u8],
) -> Result<Cow<'a, str>, quick_xml::Error> {
    Ok(reader.decoder().decode(bytes)?)
}

pub(crate) fn decode_text<'a, R: BufRead>(
    reader: &Reader<R>,
    text: &'a BytesText<'a>,
) -> Result<Cow<'a, str>, quick_xml::Error> {
    let cow: Cow<'a, str> = reader.decoder().decode(text)?;
    let s: &str = &cow;
    let trimmed = s.trim();
    if trimmed.len() == s.len() {
        Ok(cow)
    } else {
        Ok(Cow::Owned(trimmed.to_string()))
    }
}
