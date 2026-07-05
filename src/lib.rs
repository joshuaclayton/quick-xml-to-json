#![forbid(unsafe_code)]

mod decoders;
mod errors;
mod frames;

use crate::frames::AttributesWriter;
pub use errors::XmlToJsonError;
use quick_xml::Reader;
use quick_xml::events::Event;
use std::io::{BufRead, BufReader, Read, Write};

const MB: usize = 1024 * 1024;

/// Convert XML to JSON
///
/// # Example Usage
///
/// ```
/// use quick_xml_to_json::xml_to_json;
///
/// let xml = r#"<root><parent id="1"><child>Value</child></parent></root>"#;
/// let expected_json = serde_json::json!({
///     "root": {
///         "#c": [
///             {
///                 "parent": {
///                     "@id": "1",
///                     "#c": [
///                         { "child": { "#t": "Value" } }
///                     ]
///                 }
///             }
///         ]
///     }
/// });
///
///
/// let mut output = Vec::new();
/// assert!(xml_to_json(xml.as_bytes(), &mut output).is_ok());
///
/// assert_eq!(
///   expected_json,
///   serde_json::from_slice::<serde_json::Value>(&output).unwrap()
/// );
/// ```
///
/// # Errors
///
/// This may error when:
///
/// * reading XML
/// * encountering a malformed character reference (e.g. `&#xZZ;`)
/// * writing to the output
pub fn xml_to_json<R: Read, W: Write>(reader: R, out: W) -> Result<(), XmlToJsonError> {
    xml_to_json_from_bufread(BufReader::new(reader), out)
}

/// The shared event loop for both reader modes.
///
/// This is a macro rather than a function because the two modes have incompatible
/// borrow shapes: buffered events borrow from a per-iteration scratch buffer, while
/// slice events borrow from the input for its whole lifetime.
macro_rules! convert_events {
    ($xml:ident, $writer:ident, $next_event:expr) => {{
        let mut stack: Vec<frames::Element> = Vec::with_capacity(16);
        let mut spare_text_buf = String::new();

        loop {
            match ($next_event, stack.last_mut()) {
                // # Process root element
                //
                // Open root element that has children
                (Event::Start(e), None) => {
                    let text_buf = std::mem::take(&mut spare_text_buf);
                    let mut frame = frames::Element::new_and_open(&e, &mut $writer, text_buf)?;
                    frame.process_element_attributes(&e, &mut $writer)?;

                    stack.push(frame);
                }

                // Open root that has no children
                (Event::Empty(e), None) => {
                    let mut frame = frames::EmptyNode::new_and_open(&e, &mut $writer)?;
                    frame.process_element_attributes(&e, &mut $writer)?;
                    frame.close(&mut $writer)?;

                    $writer.flush()?;
                    return Ok(());
                }

                // # Process child element
                //
                // Open child element that has children
                (Event::Start(e), Some(parent)) => {
                    parent.begin_child(&mut $writer)?;

                    let text_buf = std::mem::take(&mut spare_text_buf);
                    let mut frame = frames::Element::new_and_open(&e, &mut $writer, text_buf)?;
                    frame.process_element_attributes(&e, &mut $writer)?;

                    stack.push(frame);
                }

                // Open child element that has no children
                (Event::Empty(e), Some(parent)) => {
                    parent.begin_child(&mut $writer)?;

                    let mut frame = frames::EmptyNode::new_and_open(&e, &mut $writer)?;
                    frame.process_element_attributes(&e, &mut $writer)?;
                    frame.close(&mut $writer)?;
                }

                // Process a text node of an element
                //
                // Whitespace-only events before any real content would be edge-trimmed away
                // anyway, so skip them on the raw bytes without paying for UTF-8 decoding —
                // pretty-printed documents are full of them.
                (Event::Text(t), Some(frame)) => {
                    if frame.has_text()
                        || !t.iter().all(|b| matches!(b, b' ' | b'\t' | b'\r' | b'\n'))
                    {
                        let text = decoders::decode_bytes(&t)?;
                        frame.push_text(text);
                    }
                }

                // Process a reference (`&amp;`, `&#65;`, ...) within an element's text.
                //
                // Character references and predefined entities resolve to their characters;
                // named entities we cannot resolve (e.g. DTD-defined) pass through verbatim.
                (Event::GeneralRef(r), Some(frame)) => {
                    if let Some(ch) = r.resolve_char_ref()? {
                        frame.push_char(ch);
                    } else {
                        let name = decoders::decode_bytes(&r)?;
                        match quick_xml::escape::resolve_predefined_entity(name) {
                            Some(resolved) => frame.push_resolved(resolved),
                            None => frame.push_unresolved_ref(name),
                        }
                    }
                }

                // Close out the current node on the stack
                (Event::End(_), _) => {
                    if let Some(mut frame) = stack.pop() {
                        frame.close(&mut $writer)?;
                        spare_text_buf = frame.take_text_buf();

                        // If there's nothing else on the stack, we're done
                        if stack.is_empty() {
                            $writer.flush()?;
                            return Ok(());
                        }
                    }
                }

                (Event::Eof, _) => break,
                _ => {}
            }
        }

        Err(XmlToJsonError::InvalidXML)
    }};
}

/// Convert XML to JSON from a buffered reader.
///
/// Use this instead of [`xml_to_json`] when the input is already buffered (e.g. a
/// `BufReader` you manage yourself) to avoid double-buffering. If the whole document
/// is already in memory, prefer [`xml_to_json_from_slice`], which avoids copying each
/// event into an intermediate buffer.
///
/// # Errors
///
/// This may error when:
///
/// * reading XML
/// * encountering a malformed character reference (e.g. `&#xZZ;`)
/// * writing to the output
pub fn xml_to_json_from_bufread<R: BufRead, W: Write>(
    reader: R,
    out: W,
) -> Result<(), XmlToJsonError> {
    let mut writer = std::io::BufWriter::with_capacity(MB * 2, out);
    let mut xml = Reader::from_reader(reader);
    let mut buf = Vec::with_capacity(256);

    convert_events!(xml, writer, {
        buf.clear();
        xml.read_event_into(&mut buf)?
    })
}

/// Convert XML to JSON from an in-memory byte slice.
///
/// Use this instead of [`xml_to_json`] when the whole document is already in memory:
/// events borrow directly from the input, avoiding the copy of every event into an
/// intermediate buffer that the reader-based APIs must perform.
///
/// # Example Usage
///
/// ```
/// use quick_xml_to_json::xml_to_json_from_slice;
///
/// let xml = r#"<root><child>Value</child></root>"#;
/// let mut output = Vec::new();
/// assert!(xml_to_json_from_slice(xml.as_bytes(), &mut output).is_ok());
/// ```
///
/// # Errors
///
/// This may error when:
///
/// * reading XML
/// * encountering a malformed character reference (e.g. `&#xZZ;`)
/// * writing to the output
pub fn xml_to_json_from_slice<W: Write>(input: &[u8], out: W) -> Result<(), XmlToJsonError> {
    let mut writer = std::io::BufWriter::with_capacity(MB * 2, out);
    let mut xml = Reader::from_reader(input);

    convert_events!(xml, writer, xml.read_event()?)
}

#[cfg(test)]
mod tests {

    #[test]
    fn test_nested_structure() {
        let xml = r#"
            <root>
                <child1 attr1="value1">
                    <subchild>Text 1</subchild>
                    <subchild>Text 2</subchild>
                    <subchild>Text 3</subchild>
                </child1>
                <child2 attr2="value2" />
                <child1 attr2="value2" attr3="value3" attr1="value1">
                    <subchild>Text 2</subchild>
                    <subchild>Text 1</subchild>
                    <subchild>Text 3</subchild>
                </child1>
            </root>
        "#;

        let expected_json = serde_json::json!({
            "root": {
                "#c": [
                    {
                        "child1": {
                            "@attr1": "value1",
                            "#c": [
                                {
                                    "subchild": {
                                        "#t": "Text 1"
                                    }
                                },
                                {
                                    "subchild": {
                                        "#t": "Text 2"
                                    }
                                },
                                {
                                    "subchild": {
                                        "#t": "Text 3"
                                    }
                                },
                            ]
                        }
                    },
                    {
                        "child2": {
                            "@attr2": "value2"
                        }
                    },
                    {
                        "child1": {
                            "@attr3": "value3",
                            "@attr2": "value2",
                            "@attr1": "value1",
                            "#c": [
                                {
                                    "subchild": {
                                        "#t": "Text 2"
                                    }
                                },
                                {
                                    "subchild": {
                                        "#t": "Text 1"
                                    }
                                },
                                {
                                    "subchild": {
                                        "#t": "Text 3"
                                    }
                                },
                            ]
                        }
                    },
                ]
            }
        });

        assert_eq!(expected_json, convert_xml_to_json(xml));
    }

    #[test]
    fn test_basic_xml_to_json() {
        let xml = r#"<users count="3">
  <user age="40">Jane Doe</user>
  <user age="42">John Doe</user>
  <user age="12">Jim Doe</user>
</users>"#;

        let expected_json = serde_json::json!({
            "users": {
                "@count": "3",
                "#c": [
                    {
                        "user": {
                            "@age": "40",
                            "#t": "Jane Doe"
                        }
                    },
                    {
                        "user": {
                            "@age": "42",
                            "#t": "John Doe"
                        }
                    },
                    {
                        "user": {
                            "@age": "12",
                            "#t": "Jim Doe"
                        }
                    }
                ]
            }
        });

        assert_eq!(expected_json, convert_xml_to_json(xml));
    }

    #[test]
    fn test_single_element_with_text() {
        let xml = "<name>John Doe</name>";
        let expected_json = serde_json::json!({
            "name": {
                "#t": "John Doe"
            }
        });

        assert_eq!(expected_json, convert_xml_to_json(xml));
    }

    #[test]
    fn test_element_with_attributes_only() {
        let xml = r#"<div class="container" id="main"></div>"#;
        let expected_json = serde_json::json!({
            "div": {
                "@class": "container",
                "@id": "main"
            }
        });

        assert_eq!(expected_json, convert_xml_to_json(xml));
    }

    #[test]
    fn test_nested_elements() {
        let xml = r#"<root><parent id="1"><child>Value</child></parent></root>"#;
        let expected_json = serde_json::json!({
            "root": {
                "#c": [
                    {
                        "parent": {
                            "@id": "1",
                            "#c": [
                                { "child": { "#t": "Value" } }
                            ]
                        }
                    }
                ]
            }
        });

        assert_eq!(expected_json, convert_xml_to_json(xml));
    }

    #[test]
    fn test_mixed_content_trailing_text_is_kept_and_does_not_leak() {
        let xml = "<r><a>pre <b/>post</a><c>text</c></r>";
        let expected_json = serde_json::json!({
            "r": {
                "#c": [
                    {
                        "a": {
                            "#c": [ { "b": {} } ],
                            "#t": "pre post"
                        }
                    },
                    {
                        "c": { "#t": "text" }
                    }
                ]
            }
        });

        assert_eq!(expected_json, convert_xml_to_json(xml));
    }

    #[test]
    fn test_text_after_children_only() {
        let xml = "<a><b/>tail</a>";
        let expected_json = serde_json::json!({
            "a": {
                "#c": [ { "b": {} } ],
                "#t": "tail"
            }
        });

        assert_eq!(expected_json, convert_xml_to_json(xml));
    }

    #[test]
    fn test_predefined_entities_in_text() {
        let xml = "<a>x &amp; y &lt;tag&gt; &quot;q&quot; &apos;s&apos;</a>";
        let expected_json = serde_json::json!({
            "a": { "#t": r#"x & y <tag> "q" 's'"# }
        });

        assert_eq!(expected_json, convert_xml_to_json(xml));
    }

    #[test]
    fn test_entities_do_not_introduce_spaces() {
        let xml = "<a>a&amp;b</a>";
        let expected_json = serde_json::json!({
            "a": { "#t": "a&b" }
        });

        assert_eq!(expected_json, convert_xml_to_json(xml));
    }

    #[test]
    fn test_numeric_char_refs_in_text() {
        let xml = "<a>&#72;&#x65;y</a>";
        let expected_json = serde_json::json!({
            "a": { "#t": "Hey" }
        });

        assert_eq!(expected_json, convert_xml_to_json(xml));
    }

    #[test]
    fn test_char_ref_producing_json_escapable_char() {
        let xml = "<a>x&#10;y</a>";
        let expected_json = serde_json::json!({
            "a": { "#t": "x\ny" }
        });

        assert_eq!(expected_json, convert_xml_to_json(xml));
    }

    #[test]
    fn test_unknown_named_entity_passes_through_in_text() {
        let xml = "<a>x &uuml; y</a>";
        let expected_json = serde_json::json!({
            "a": { "#t": "x &uuml; y" }
        });

        assert_eq!(expected_json, convert_xml_to_json(xml));
    }

    #[test]
    fn test_invalid_char_ref_in_text_errors() {
        assert!(super::xml_to_json(b"<a>&#xZZ;</a>".as_slice(), Vec::new()).is_err());
        assert!(super::xml_to_json(b"<a>&#+65;</a>".as_slice(), Vec::new()).is_err());
    }

    #[test]
    fn test_entities_in_attribute_values() {
        let xml = r#"<a m="5 &lt; 6" q="&quot;q&quot;" n="A&#66;C" u="x &uuml; y"/>"#;
        let expected_json = serde_json::json!({
            "a": {
                "@m": "5 < 6",
                "@q": "\"q\"",
                "@n": "ABC",
                "@u": "x &uuml; y"
            }
        });

        assert_eq!(expected_json, convert_xml_to_json(xml));
    }

    #[test]
    fn test_invalid_char_ref_in_attribute_errors() {
        assert!(super::xml_to_json(br#"<a t="&#xZZ;"/>"#.as_slice(), Vec::new()).is_err());
        assert!(super::xml_to_json(br#"<a t="&#+65;"/>"#.as_slice(), Vec::new()).is_err());
    }

    #[test]
    fn test_bare_ampersand_in_attribute_passes_through() {
        let xml = r#"<a href="q?x=1&y=2"/>"#;
        let expected_json = serde_json::json!({
            "a": { "@href": "q?x=1&y=2" }
        });

        assert_eq!(expected_json, convert_xml_to_json(xml));
    }

    #[test]
    fn test_interior_whitespace_preserved_in_text() {
        let xml = "<a>line1\nline2</a>";
        let expected_json = serde_json::json!({
            "a": { "#t": "line1\nline2" }
        });

        assert_eq!(expected_json, convert_xml_to_json(xml));
    }

    #[test]
    fn test_edge_whitespace_trimmed_from_text() {
        let xml = "<a>\n  padded  \n</a>";
        let expected_json = serde_json::json!({
            "a": { "#t": "padded" }
        });

        assert_eq!(expected_json, convert_xml_to_json(xml));
    }

    #[test]
    fn test_whitespace_only_text_produces_no_text_node() {
        let xml = "<a>   </a>";
        let expected_json = serde_json::json!({ "a": {} });

        assert_eq!(expected_json, convert_xml_to_json(xml));
    }

    #[test]
    fn test_empty_xml() {
        assert!(super::xml_to_json(b"".as_slice(), Vec::new()).is_err());
    }

    #[test]
    fn test_malformed_xml() {
        assert!(super::xml_to_json(b"<root><unclosed>".as_slice(), Vec::new()).is_err());
        assert!(super::xml_to_json_from_slice(b"<root><unclosed>", Vec::new()).is_err());
        assert!(super::xml_to_json_from_slice(b"", Vec::new()).is_err());
    }

    #[test]
    fn test_empty_nodes() {
        let xml = "<main><br /></main>";
        let expected_json = serde_json::json!({
            "main": {
                "#c": [
                    { "br": {}}
                ]
            }
        });

        assert_eq!(expected_json, convert_xml_to_json(xml));
    }

    #[test]
    fn test_empty_node_at_root() {
        let xml = "<br />";
        let expected_json = serde_json::json!({ "br": {} });

        assert_eq!(expected_json, convert_xml_to_json(xml));
    }

    #[test]
    fn test_empty_node_at_root_with_attributes() {
        let xml = r#"<br id="5" />"#;
        let expected_json = serde_json::json!({ "br": { "@id": "5" } });

        assert_eq!(expected_json, convert_xml_to_json(xml));
    }

    fn convert_xml_to_json(xml: &str) -> serde_json::Value {
        let mut output = Vec::new();
        super::xml_to_json(xml.as_bytes(), &mut output).unwrap();

        let mut slice_output = Vec::new();
        super::xml_to_json_from_slice(xml.as_bytes(), &mut slice_output).unwrap();
        assert_eq!(
            output, slice_output,
            "slice and bufread APIs must produce identical output"
        );

        serde_json::from_slice(&output).unwrap()
    }
}
