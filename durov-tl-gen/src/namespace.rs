use crate::context::Context;
use crate::write::Write;
use crate::writer::Writer;

pub struct Namespace<T> {
    pub name: Option<String>,
    pub items: Vec<T>,
}

impl<T: Write> Write for Namespace<T> {
    fn write(&self, writer: &mut Writer, context: &mut Context) {
        if let Some(name) = &self.name {
            writer.indent_write("pub mod ");
            writer.raw_write(name);
            writer.raw_write(" {\n");
            writer.add_indent();
            context.nested += 1;
        }
        for item in &self.items {
            item.write(writer, context);
        }
        if self.name.is_some() {
            writer.subtract_indent();
            writer.indent_write("}\n\n");
            context.nested -= 1;
        }
    }
}
