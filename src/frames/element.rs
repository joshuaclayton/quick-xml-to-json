use super::buffers::{self, write_json_string, write_json_string_unchecked};
use quick_xml::{Reader, events::BytesStart};
use std::io::Write;

pub(crate) struct Element {
    // Tag name
    pub(crate) name: String,

    // Are we operating on the first field of this element?
    //
    // This determines whether we need to insert a comma before the next field
    first_field: bool,

    // Is the array of children currently open?
    //
    // This is set to true once we begin processing children and is set to false when closing out
    // this element entirely.
    children_open: bool,

    // Are we operating on the first child element?
    //
    // Similar to `first_field`, we need to track state around whether we're writing the first
    // child or not. If we aren't on the first child, we need to insert a comma before the next
    // child gets written.
    first_child: bool,

    // Accumulate text nodes until we either see a child or close
    text_buf: String,
}

impl super::AttributesWriter for Element {
    fn first_field(&self) -> bool {
        self.first_field
    }

    fn process_first_field(&mut self) {
        self.first_field = false;
    }
}

impl Element {
    pub(crate) fn from_element<R: std::io::BufRead>(
        e: &BytesStart,
        xml: &Reader<R>,
    ) -> Result<Self, quick_xml::Error> {
        let qname = e.name();
        let tag = crate::decoders::decode_bytes(xml, qname.as_ref())?;

        Ok(Self {
            name: tag.into_owned(),
            first_field: true,
            children_open: false,
            first_child: true,
            text_buf: String::new(),
        })
    }

    /// Write the wrapper and open the element object: {"name":{
    pub(crate) fn open<W: Write>(&mut self, mut w: W) -> Result<(), buffers::BufferError> {
        w.write_all(b"{")?;
        write_json_string(&self.name, &mut w)?;
        w.write_all(b":{")?;

        Ok(())
    }

    /// Buffer text until we either see children or close.
    pub(crate) fn push_text(&mut self, s: &str) {
        if s.is_empty() {
            return;
        }

        if !self.text_buf.is_empty() {
            self.text_buf.push(' ');
        }

        self.text_buf.push_str(s);
    }

    /// Flush buffered text as "#t":"..."
    fn flush_text<W: Write>(&mut self, mut w: W) -> Result<(), buffers::BufferError> {
        if self.text_buf.is_empty() {
            return Ok(());
        }
        if !self.first_field {
            w.write_all(b",")?;
        }
        write_json_string_unchecked(crate::TEXT_NODE_KEY, &mut w)?;
        w.write_all(b":")?;
        write_json_string(&self.text_buf, &mut w)?;

        self.first_field = false;
        self.text_buf.clear();

        Ok(())
    }

    /// Ensure "#c":[ is opened
    fn ensure_children_open<W: Write>(&mut self, mut w: W) -> Result<(), buffers::BufferError> {
        if !self.children_open {
            self.flush_text(&mut w)?;
            if !self.first_field {
                w.write_all(b",")?;
            }
            write_json_string_unchecked(crate::CHILDREN_KEY, &mut w)?;
            w.write_all(b":[")?;
            self.children_open = true;
            self.first_field = false;
            self.first_child = true;
        }

        Ok(())
    }

    /// Begin a child entry inside #c: write comma if needed
    pub(crate) fn begin_child<W: Write>(&mut self, mut w: W) -> Result<(), buffers::BufferError> {
        self.ensure_children_open(&mut w)?;

        if !self.first_child {
            w.write_all(b",")?;
        }
        self.first_child = false;

        Ok(())
    }

    /// Close current element; if #c was opened, close it as well.
    pub(crate) fn close<W: Write>(&mut self, mut w: W) -> Result<(), buffers::BufferError> {
        // if no children were opened, we still need to flush any text
        if self.children_open {
            w.write_all(b"]")?; // close #c
            self.children_open = false;
        } else {
            self.flush_text(&mut w)?;
        }

        w.write_all(b"}}")?;

        Ok(())
    }
}
