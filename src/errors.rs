use crate::frames::buffers;

#[derive(Debug, thiserror::Error)]
pub enum XmlToJsonError {
    #[error("QuickXML Error: {0}")]
    QuickXml(#[from] quick_xml::Error),

    #[error("QuickXML Attribute Error: {0}")]
    QuickXmlAttribute(#[from] quick_xml::events::attributes::AttrError),

    #[error("SerdeJSON Error: {0}")]
    SerdeJson(#[from] serde_json::Error),

    #[error("Buffer Error: {0}")]
    Buffer(#[from] buffers::BufferError),

    #[error("IO Error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Malformed XML")]
    InvalidXML,
}
