use crate::context::Context;
use crate::write::Write;
use crate::writer::Writer;
use convert_case::{Case, Casing};
use durov_tl_parser::{Combinator, CombinatorType, DataType, Field};

const RESERVED_KEYWORDS: &[&str] = &[
    "as",
    "break",
    "const",
    "continue",
    "crate",
    "else",
    "enum",
    "extern",
    "false",
    "fn",
    "for",
    "if",
    "impl",
    "in",
    "let",
    "loop",
    "match",
    "mod",
    "move",
    "mut",
    "pub",
    "ref",
    "return",
    "self",
    "Self",
    "static",
    "struct",
    "super",
    "trait",
    "true",
    "type",
    "unsafe",
    "use",
    "where",
    "while",
    "async",
    "await",
    "dyn",
    "abstract",
    "become",
    "box",
    "do",
    "final",
    "macro",
    "override",
    "priv",
    "typeof",
    "unsized",
    "virtual",
    "yield",
    "try",
    "gen",
];

impl Write for Combinator {
    fn write(&self, writer: &mut Writer, context: &mut Context) {
        let has_generic = self.fields.iter()
            .any(|f| f.typ == DataType::Function);

        writer.indent_write("#[derive(Debug, Clone, PartialEq)]\n");
        writer.indent_write("pub struct ");
        writer.raw_write(&self.name.name.to_case(Case::Pascal));
        if has_generic {
            writer.raw_write("<F>");
        }
        writer.raw_write(" {\n");
        writer.add_indent();
        for field in &self.fields {
            if matches!(field.typ, DataType::Condition) {
                continue;
            }
            writer.indent_write("pub ");
            writer.raw_write(&no_reserved_keyword(&field.name));
            writer.raw_write(": ");
            field.typ.write(writer, context);
            writer.raw_write(",\n");
        }
        writer.subtract_indent();
        writer.indent_write("}\n\n");

        writer.indent_write("impl");
        if has_generic {
            writer.raw_write("<F>");
        }
        writer.raw_write(" crate::Identify for ");
        writer.raw_write(&self.name.name.to_case(Case::Pascal));
        if has_generic {
            writer.raw_write("<F>");
        }
        writer.raw_write(" {\n");
        writer.add_indent();
        writer.indent_write("const ID: i32 = ");
        writer.raw_write(&self.id.to_string());
        writer.raw_write(";\n");
        writer.subtract_indent();
        writer.indent_write("}\n\n");

        if self.typ == CombinatorType::Function {
            writer.indent_write("impl");
            if has_generic {
                writer.raw_write("<F: crate::Call>");
            }
            writer.raw_write(" crate::Call for ");
            writer.raw_write(&self.name.name.to_case(Case::Pascal));
            if has_generic {
                writer.raw_write("<F>");
            }
            writer.raw_write(" {\n");
            writer.add_indent();
            writer.indent_write("type Result = ");
            self.data_type.write(writer, context);
            writer.raw_write(";\n");
            writer.subtract_indent();
            writer.indent_write("}\n\n");
        }

        writer.indent_write("impl");
        if has_generic {
            writer.raw_write("<F: crate::serialize::Serialize>");
        }
        writer.raw_write(" crate::serialize::Serialize for ");
        writer.raw_write(&self.name.name.to_case(Case::Pascal));
        if has_generic {
            writer.raw_write("<F>");
        }
        writer.raw_write(" {\n");
        writer.add_indent();
        writer.indent_write("fn serialize(&self, ");
        if self.typ == CombinatorType::Constructor && self.fields.is_empty() {
            writer.raw_write("_");
        }
        writer.raw_write("dst: &mut crate::buffer::Buffer) {\n");
        writer.add_indent();
        if self.typ == CombinatorType::Function {
            writer.indent_write("<Self as crate::Identify>::ID.serialize(dst);\n");
        }
        for field in &self.fields {
            match field.typ {
                DataType::Condition => {
                    let cond_fields = collect_conditional_fields(&self.fields, &field.name);
                    writer.indent_write("{ 0 ");
                    if !cond_fields.is_empty() {
                        writer.raw_write("| ");
                    }
                    for (idx, (name, bit, bool)) in cond_fields.iter().enumerate() {
                        writer.raw_write("(self.");
                        writer.raw_write(&no_reserved_keyword(name));
                        if !bool {
                            writer.raw_write(".is_some()");
                        }
                        writer.raw_write(" as i32) << ");
                        writer.raw_write(&bit.to_string());
                        writer.raw_write(" ");
                        if idx + 1 < cond_fields.len() {
                            writer.raw_write("| ");
                        }
                    }
                    writer.raw_write("}.serialize(dst);\n");
                }
                DataType::Conditional { .. } => {
                    writer.indent_write("if let Some(");
                    writer.raw_write(&field.name);
                    writer.raw_write("_) = &self.");
                    writer.raw_write(&no_reserved_keyword(&field.name));
                    writer.raw_write(" {\n");
                    writer.add_indent();
                    writer.indent_write(&field.name);
                    writer.raw_write("_.serialize(dst);\n");
                    writer.subtract_indent();
                    writer.indent_write("}\n");
                }
                DataType::ConditionalTrue { .. } => {}
                _ => {
                    writer.indent_write("self.");
                    writer.raw_write(&no_reserved_keyword(&field.name));
                    writer.raw_write(".serialize(dst);\n");
                }
            }
        }
        writer.subtract_indent();
        writer.indent_write("}\n");
        writer.subtract_indent();
        writer.indent_write("}\n\n");

        if self.typ == CombinatorType::Constructor {
            writer.indent_write("impl");
            writer.raw_write(" crate::deserialize::Deserialize for ");
            writer.raw_write(&self.name.name.to_case(Case::Pascal));
            writer.raw_write(" {\n");
            writer.add_indent();
            writer.indent_write("fn deserialize(");
            if self.fields.is_empty() {
                writer.raw_write("_");
            }
            writer.raw_write("src: &mut crate::cursor::Cursor) -> Result<Self, crate::deserialize::Error> {\n");
            writer.add_indent();
            for field in &self.fields {
                writer.indent_write("let ");
                if matches!(field.typ, DataType::Condition) {
                    let cond_fields = collect_conditional_fields(&self.fields, &field.name);
                    if cond_fields.is_empty() {
                        writer.raw_write("_");
                    }
                }
                writer.raw_write(&field.name);
                match &field.typ {
                    DataType::Conditional { field, bit, .. } => {
                        writer.raw_write("_ = if ");
                        writer.raw_write(field);
                        writer.raw_write("_ & (1 << ");
                        writer.raw_write(&bit.to_string());
                        writer.raw_write(") != 0 { Some(crate::deserialize::Deserialize::deserialize(src)?) } else { None };\n");
                    }
                    DataType::ConditionalTrue { field, bit } => {
                        writer.raw_write("_ = ");
                        writer.raw_write(field);
                        writer.raw_write("_ & (1 << ");
                        writer.raw_write(&bit.to_string());
                        writer.raw_write(") != 0;\n");
                    }
                    _ => {
                        writer.raw_write("_ = ");
                        field.typ.write(writer, context);
                        writer.raw_write("::deserialize(src)?;\n");
                    }
                }
            }
            writer.indent_write("Ok(Self { ");
            for field in &self.fields {
                if matches!(field.typ, DataType::Condition) {
                    continue;
                }
                writer.raw_write(&no_reserved_keyword(&field.name));
                writer.raw_write(": ");
                writer.raw_write(&field.name);
                writer.raw_write("_, ");
            }
            writer.raw_write("})\n");
            writer.subtract_indent();
            writer.indent_write("}\n");
            writer.subtract_indent();
            writer.indent_write("}\n\n");
        }
    }
}

fn collect_conditional_fields(fields: &[Field], depend_on: &str) -> Vec<(String, u8, bool)> {
    fields.iter()
        .filter_map(|f| match &f.typ {
            DataType::Conditional { field, bit, .. } if field == depend_on => {
                Some((f.name.clone(), *bit, false))
            }
            DataType::ConditionalTrue { field, bit } if field == depend_on => {
                Some((f.name.clone(), *bit, true))
            }
            _ => None,
        })
        .collect()
}

fn no_reserved_keyword(name: &str) -> String {
    if RESERVED_KEYWORDS.contains(&name) {
        format!("{name}_")
    } else {
        name.to_string()
    }
}
