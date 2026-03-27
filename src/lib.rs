#![forbid(unsafe_code)]

mod decoders;
mod errors;
mod frames;

use crate::frames::AttributesWriter;
use decoders::decode_text;
pub use errors::XmlToJsonError;
use quick_xml::Reader;
use quick_xml::events::Event;
use std::io::{BufRead, BufReader, Read, Write};

static CHILDREN_KEY: &str = "#c";
static TEXT_NODE_KEY: &str = "#t";
static MB: usize = 1024 * 1024;

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
/// * serializing strings to JSON
/// * converting a String to a byte array
/// * writing to the buffer
pub fn xml_to_json<R: Read, W: Write>(reader: R, out: W) -> Result<(), XmlToJsonError> {
    xml_to_json_from_bufread(BufReader::new(reader), out)
}

/// Convert XML to JSON from a buffered reader.
///
/// Use this instead of [`xml_to_json`] when the input is already buffered (e.g. `BufReader`,
/// `Cursor`, or an in-memory byte slice via `std::io::Cursor`) to avoid double-buffering.
///
/// # Errors
///
/// This may error when:
///
/// * reading XML
/// * serializing strings to JSON
/// * converting a String to a byte array
/// * writing to the buffer
pub fn xml_to_json_from_bufread<R: BufRead, W: Write>(
    reader: R,
    out: W,
) -> Result<(), XmlToJsonError> {
    let mut writer = std::io::BufWriter::with_capacity(MB * 2, out);
    let mut xml = Reader::from_reader(reader);
    xml.config_mut().trim_text(true);
    let mut buf = Vec::with_capacity(256);
    let mut stack: Vec<frames::Element> = Vec::with_capacity(16);
    let mut spare_text_buf = String::new();

    loop {
        match (xml.read_event_into(&mut buf)?, stack.last_mut()) {
            // # Process root element
            //
            // Open root element that has children
            (Event::Start(e), None) => {
                let text_buf = std::mem::take(&mut spare_text_buf);
                let mut frame = frames::Element::new_and_open(&e, &xml, &mut writer, text_buf)?;
                frame.process_element_attributes(&e, &xml, &mut writer)?;

                stack.push(frame);
            }

            // Open root that has no children
            (Event::Empty(e), None) => {
                let mut frame = frames::EmptyNode::new_and_open(&e, &xml, &mut writer)?;
                frame.process_element_attributes(&e, &xml, &mut writer)?;
                frame.close(&mut writer)?;

                writer.flush()?;
                return Ok(());
            }

            // # Process child element
            //
            // Open child element that has children
            (Event::Start(e), Some(parent)) => {
                parent.begin_child(&mut writer)?;

                let text_buf = std::mem::take(&mut spare_text_buf);
                let mut frame = frames::Element::new_and_open(&e, &xml, &mut writer, text_buf)?;
                frame.process_element_attributes(&e, &xml, &mut writer)?;

                stack.push(frame);
            }

            // Open child element that has no children
            (Event::Empty(e), Some(parent)) => {
                parent.begin_child(&mut writer)?;

                let mut frame = frames::EmptyNode::new_and_open(&e, &xml, &mut writer)?;
                frame.process_element_attributes(&e, &xml, &mut writer)?;
                frame.close(&mut writer)?;
            }

            // Process a text node of an element
            (Event::Text(t), Some(frame)) => {
                let text = decode_text(&xml, &t)?;
                frame.push_text(&text);
            }

            // Close out the current node on the stack
            (Event::End(_), _) => {
                if let Some(mut frame) = stack.pop() {
                    frame.close(&mut writer)?;
                    spare_text_buf = frame.take_text_buf();

                    // If there's nothing else on the stack, we're done
                    if stack.is_empty() {
                        writer.flush()?;
                        return Ok(());
                    }
                }
            }

            (Event::Eof, _) => break,
            _ => {}
        }

        buf.clear();
    }

    Err(XmlToJsonError::InvalidXML)
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
    fn test_empty_xml() {
        assert!(super::xml_to_json(b"".as_slice(), Vec::new()).is_err());
    }

    #[test]
    fn test_malformed_xml() {
        assert!(super::xml_to_json(b"<root><unclosed>".as_slice(), Vec::new()).is_err());
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

        serde_json::from_slice(&output).unwrap()
    }
}
