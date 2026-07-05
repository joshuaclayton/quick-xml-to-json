use super::buffers::write_tag_open;
use crate::XmlToJsonError;
use quick_xml::events::BytesStart;
use std::io::{self, Write};

pub struct EmptyNode {
    // Are we operating on the first field of this element?
    //
    // This determines whether we need to insert a comma before the next field
    first_field: bool,
}

impl super::AttributesWriter for EmptyNode {
    fn first_field(&self) -> bool {
        self.first_field
    }

    fn process_first_field(&mut self) {
        self.first_field = false;
    }
}

impl EmptyNode {
    /// Create the node and immediately write the opening `{"name":{` from the borrowed tag.
    pub(crate) fn new_and_open<W: Write>(e: &BytesStart, mut w: W) -> Result<Self, XmlToJsonError> {
        let qname = e.name();
        let tag = crate::decoders::decode_bytes(qname.as_ref())?;
        write_tag_open(tag, &mut w)?;

        Ok(Self { first_field: true })
    }

    /// Close current element
    #[expect(clippy::unused_self, reason = "method consistency with Element::close")]
    pub(crate) fn close<W: Write>(&self, mut w: W) -> io::Result<()> {
        w.write_all(b"}}")?;

        Ok(())
    }
}
