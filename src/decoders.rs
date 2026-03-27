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
