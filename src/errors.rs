#[non_exhaustive]
#[derive(Debug, thiserror::Error)]
pub enum XmlToJsonError {
    #[error("QuickXML Error: {0}")]
    QuickXml(#[from] quick_xml::Error),

    #[error("QuickXML Attribute Error: {0}")]
    QuickXmlAttribute(#[from] quick_xml::events::attributes::AttrError),

    #[error("IO Error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Malformed XML")]
    InvalidXML,

    #[error("Invalid character reference: &{0};")]
    InvalidCharRef(String),
}
