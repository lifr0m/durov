use crate::write::{Context, Write};
use crate::writer::Writer;
use convert_case::{Case, Casing};
use durov_tl_parser::Name;

impl Write for Name {
    fn write(&self, writer: &mut Writer, context: &mut Context) {
        for _ in 0..context.nested {
            writer.raw_write("super::");
        }
        writer.raw_write(&context.namespaces[self]);
        writer.raw_write("::");
        if let Some(namespace) = &self.namespace {
            writer.raw_write(namespace);
            writer.raw_write("::");
        }
        writer.raw_write(&self.name.to_case(Case::Pascal));
    }
}
