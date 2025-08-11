use super::buffers::{self, write_json_string};
use quick_xml::{Reader, events::BytesStart};
use std::io::Write;

pub(crate) struct EmptyNode {
    // Tag name
    pub(super) name: String,

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
    pub(crate) fn from_element<R: std::io::BufRead>(
        e: &BytesStart,
        xml: &Reader<R>,
    ) -> Result<Self, quick_xml::Error> {
        let qname = e.name();
        let tag = crate::decoders::decode_bytes(xml, qname.as_ref())?;

        Ok(Self {
            name: tag.into_owned(),
            first_field: true,
        })
    }

    /// Write the wrapper and open the element object: {"name":{
    pub(crate) fn open<W: Write>(&self, mut w: W) -> Result<(), buffers::BufferError> {
        w.write_all(b"{")?;
        write_json_string(&self.name, &mut w)?;
        w.write_all(b":{")?;

        Ok(())
    }

    /// Close current element
    #[allow(clippy::unused_self)]
    pub(crate) fn close<W: Write>(&self, mut w: W) -> Result<(), buffers::BufferError> {
        w.write_all(b"}}")?;

        Ok(())
    }
}
