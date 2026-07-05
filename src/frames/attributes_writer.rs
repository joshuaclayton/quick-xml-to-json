use super::buffers::{write_json_string, write_json_string_with_prefix_unchecked};
use crate::{
    XmlToJsonError,
    decoders::{decode_bytes, unescape_lenient},
};
use quick_xml::{Reader, events::BytesStart};
use std::io::Write;

pub trait AttributesWriter {
    fn first_field(&self) -> bool;
    fn process_first_field(&mut self);

    fn process_element_attributes<R: std::io::BufRead, W: Write>(
        &mut self,
        e: &BytesStart,
        xml: &Reader<R>,
        mut writer: W,
    ) -> Result<(), XmlToJsonError> {
        for attr in e.attributes().with_checks(false) {
            let attr = attr?;
            let key = decode_bytes(xml, attr.key.as_ref())?;
            let raw_value = decode_bytes(xml, &attr.value)?;
            let value = unescape_lenient(&raw_value)?;

            if !self.first_field() {
                writer.write_all(b",")?;
            }

            write_json_string_with_prefix_unchecked("@", &key, &mut writer)?;
            writer.write_all(b":")?;
            write_json_string(&value, &mut writer)?;
            self.process_first_field();
        }

        Ok(())
    }
}
