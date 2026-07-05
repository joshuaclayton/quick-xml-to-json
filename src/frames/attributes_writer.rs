use super::buffers::write_json_string;
use crate::{
    XmlToJsonError,
    decoders::{decode_bytes, unescape_lenient},
};
use quick_xml::events::BytesStart;
use std::io::Write;

pub trait AttributesWriter {
    fn first_field(&self) -> bool;
    fn process_first_field(&mut self);

    fn process_element_attributes<W: Write>(
        &mut self,
        e: &BytesStart,
        mut writer: W,
    ) -> Result<(), XmlToJsonError> {
        for attr in e.attributes().with_checks(false) {
            let attr = attr?;
            let key = decode_bytes(attr.key.as_ref())?;
            let raw_value = decode_bytes(&attr.value)?;
            let value = unescape_lenient(raw_value)?;

            if self.first_field() {
                writer.write_all(b"\"@")?;
            } else {
                writer.write_all(b",\"@")?;
            }
            writer.write_all(key.as_bytes())?;
            writer.write_all(b"\":")?;
            write_json_string(&value, &mut writer)?;
            self.process_first_field();
        }

        Ok(())
    }
}
