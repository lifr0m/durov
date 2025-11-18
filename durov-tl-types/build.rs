use durov_tl_gen::generate_code;
use durov_tl_gen::writer::Writer;
use durov_tl_parser::parse_schema;
use std::path::Path;

fn main() {
    let mut writer = Writer::new(4);

    for entry in std::fs::read_dir("schemas").unwrap() {
        let entry = entry.unwrap();

        let content = std::fs::read_to_string(entry.path())
            .unwrap();
        let schema = parse_schema(&content);
        let code = generate_code(&schema);

        let file_name = entry.file_name()
            .into_string()
            .unwrap();
        let name = file_name.strip_suffix(".tl")
            .unwrap();

        write_module(&mut writer, name, &code);
    }

    let out_dir = std::env::var("OUT_DIR")
        .unwrap();
    let path = Path::new(&out_dir)
        .join("schemas.rs");
    std::fs::write(path, writer.destruct())
        .unwrap();
}

fn write_module(writer: &mut Writer, name: &str, code: &str) {
    writer.indent_write("pub mod ");
    writer.raw_write(name);
    writer.raw_write(" {\n");
    writer.add_indent();
    writer.code_write(code);
    writer.subtract_indent();
    writer.indent_write("}\n\n");
}
