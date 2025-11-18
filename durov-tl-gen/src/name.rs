use crate::write::{Context, Write};
use crate::writer::Writer;
use convert_case::{Case, Casing};
use durov_tl_parser::Name;

impl Write for Name {
    fn write(&self, writer: &mut Writer, _context: &mut Context) {
        if let Some(namespace) = &self.namespace {
            writer.raw_write(namespace);
            writer.raw_write("::");
        }
        writer.raw_write(&self.name.to_case(Case::Pascal));
    }
}
