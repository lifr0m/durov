use crate::write::{Context, Write};
use crate::writer::Writer;
use convert_case::{Case, Casing};
use durov_tl_parser::{Combinator, Name};

pub struct Type<'a> {
    pub name: Name,
    pub constructors: Vec<&'a Combinator>,
}

impl<'a> Write for Type<'a> {
    fn write(&self, writer: &mut Writer, context: &mut Context) {
        writer.indent_write("#[derive(Debug)]\n");
        writer.indent_write("pub enum ");
        writer.raw_write(&self.name.name.to_case(Case::Pascal));
        writer.raw_write(" {\n");
        writer.add_indent();
        for constructor in &self.constructors {
            writer.indent_write(&constructor.name.name.to_case(Case::Pascal));
            writer.raw_write("(");
            for _ in 0..context.nested {
                writer.raw_write("super::");
            }
            writer.raw_write(&context.namespaces[&constructor.name]);
            writer.raw_write("::");
            constructor.name.write(writer, context);
            writer.raw_write("),\n");
        }
        writer.subtract_indent();
        writer.indent_write("}\n\n");

        writer.indent_write("impl crate::Serialize for ");
        writer.raw_write(&self.name.name.to_case(Case::Pascal));
        writer.raw_write(" {\n");
        writer.add_indent();
        writer.indent_write("fn serialize(&self, dst: &mut impl Extend<u8>) {\n");
        writer.add_indent();
        writer.indent_write("match self {\n");
        writer.add_indent();
        for constructor in &self.constructors {
            writer.indent_write("Self::");
            writer.raw_write(&constructor.name.name.to_case(Case::Pascal));
            writer.raw_write("(o) => {\n");
            writer.add_indent();
            writer.indent_write("<");
            for _ in 0..context.nested {
                writer.raw_write("super::");
            }
            writer.raw_write(&context.namespaces[&constructor.name]);
            writer.raw_write("::");
            constructor.name.write(writer, context);
            writer.raw_write(" as crate::Identify>::ID.serialize(dst);\n");
            writer.indent_write("o.serialize(dst);\n");
            writer.subtract_indent();
            writer.indent_write("}\n");
        }
        writer.subtract_indent();
        writer.indent_write("}\n");
        writer.subtract_indent();
        writer.indent_write("}\n");
        writer.subtract_indent();
        writer.indent_write("}\n\n");

        writer.indent_write("impl crate::Deserialize for ");
        writer.raw_write(&self.name.name.to_case(Case::Pascal));
        writer.raw_write(" {\n");
        writer.add_indent();
        writer.indent_write("fn deserialize(src: &mut crate::cursor::Cursor) -> Result<Self, crate::deserialize::Error> {\n");
        writer.add_indent();
        writer.indent_write("let id = i32::deserialize(src)?;\n");
        writer.indent_write("Ok(match id {\n");
        writer.add_indent();
        for constructor in &self.constructors {
            writer.indent_write("<");
            for _ in 0..context.nested {
                writer.raw_write("super::");
            }
            writer.raw_write(&context.namespaces[&constructor.name]);
            writer.raw_write("::");
            constructor.name.write(writer, context);
            writer.raw_write(" as crate::Identify>::ID => Self::");
            writer.raw_write(&constructor.name.name.to_case(Case::Pascal));
            writer.raw_write("(crate::Deserialize::deserialize(src)?),\n");
        }
        writer.indent_write("_ => return Err(crate::deserialize::Error::UnknownId(id)),\n");
        writer.subtract_indent();
        writer.indent_write("})\n");
        writer.subtract_indent();
        writer.indent_write("}\n");
        writer.subtract_indent();
        writer.indent_write("}\n\n");
    }
}
