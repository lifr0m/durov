pub mod writer;
mod write;
mod namespace;
mod type_info;
mod combinator;
mod data_type;
mod name;

use durov_tl_parser::{Combinator, CombinatorType, DataType, Name, Schema};
use namespace::Namespace;
use std::collections::HashMap;
use type_info::Type;
use write::{Context, Write};
use writer::Writer;

const TYPES_NAMESPACE: &str = "enums";
const CONSTRUCTORS_NAMESPACE: &str = "types";
const FUNCTIONS_NAMESPACE: &str = "functions";

pub fn generate_code(schema: &Schema) -> String {
    let mut writer = Writer::new(4);

    if let Some(layer) = schema.layer {
        write_layer(&mut writer, layer);
    }
    write_all_ids(&mut writer, schema);
    writer.raw_write("\n");

    let types = collect_types(schema);
    let mut context = Context::new(map_namespaces(schema, &types), map_types(&types));
    let types = group_types(types);
    types.write(&mut writer, &mut context);

    let constructors = collect_combinators(schema, CombinatorType::Constructor);
    let constructors = group_combinators(constructors, CombinatorType::Constructor);
    constructors.write(&mut writer, &mut context);

    let functions = collect_combinators(schema, CombinatorType::Function);
    let functions = group_combinators(functions, CombinatorType::Function);
    functions.write(&mut writer, &mut context);

    let chunk_size = 50;
    let mut chunk_indexes = Vec::new();
    for (idx, chunk) in schema.combinators.chunks(chunk_size).enumerate() {
        if chunk.iter().all(|c| c.typ == CombinatorType::Function) {
            continue;
        }
        let start = idx * chunk_size;
        let end = (idx + 1) * chunk_size;
        write_deserialize_object_chunk(&mut writer, &mut context, chunk, start, end);
        writer.raw_write("\n");
        chunk_indexes.push(idx);
    }
    write_deserialize_object(&mut writer, chunk_size, &chunk_indexes);

    writer.destruct()
}

fn write_layer(writer: &mut Writer, layer: u16) {
    writer.indent_write("pub const LAYER: i32 = ");
    writer.raw_write(&layer.to_string());
    writer.raw_write(";\n");
}

fn write_all_ids(writer: &mut Writer, schema: &Schema) {
    writer.indent_write("pub static ALL_IDS: phf::Set<i32> = phf::phf_set![");
    for combinator in &schema.combinators {
        writer.raw_write(&combinator.id.to_string());
        writer.raw_write(", ");
    }
    writer.raw_write("];\n");
}

fn write_deserialize_object(writer: &mut Writer, chunk_size: usize, chunk_indexes: &[usize]) {
    writer.indent_write("pub fn deserialize_object(src: &mut crate::cursor::Cursor) -> Result<crate::Object, crate::deserialize::Error> {\n");
    writer.add_indent();
    writer.indent_write("crate::multiple_deserialize_object(src, &[\n");
    writer.add_indent();
    for idx in chunk_indexes {
        let start = idx * chunk_size;
        let end = (idx + 1) * chunk_size;
        writer.indent_write("deserialize_object_");
        writer.raw_write(&start.to_string());
        writer.raw_write("_");
        writer.raw_write(&end.to_string());
        writer.raw_write(",\n");
    }
    writer.subtract_indent();
    writer.indent_write("])\n");
    writer.subtract_indent();
    writer.indent_write("}\n");
}

fn write_deserialize_object_chunk(
    writer: &mut Writer,
    context: &mut Context,
    combinators: &[Combinator],
    start: usize,
    end: usize,
) {
    writer.indent_write("fn deserialize_object_");
    writer.raw_write(&start.to_string());
    writer.raw_write("_");
    writer.raw_write(&end.to_string());
    writer.raw_write("(src: &mut crate::cursor::Cursor) -> Result<crate::Object, crate::deserialize::Error> {\n");
    writer.add_indent();
    writer.indent_write("let id = <i32 as crate::Deserialize>::deserialize(src)?;\n");
    writer.indent_write("src.seek(-4);\n");
    writer.indent_write("Ok(match id {\n");
    writer.add_indent();
    for combinator in combinators {
        if combinator.typ == CombinatorType::Function {
            continue;
        }
        writer.indent_write("<");
        combinator.name.write(writer, context);
        writer.raw_write(" as crate::Identify>::ID => crate::Object { id, body: Box::new(<");
        let type_name = context.combinator_type_map[&combinator.name].clone();
        type_name.write(writer, context);
        writer.raw_write(" as crate::Deserialize>::deserialize(src)?) },\n");
    }
    writer.indent_write("_ => return Err(crate::deserialize::Error::UnknownId(id)),\n");
    writer.subtract_indent();
    writer.indent_write("})\n");
    writer.subtract_indent();
    writer.indent_write("}\n");
}

fn collect_types(schema: &Schema) -> Vec<Type<'_>> {
    let mut map = HashMap::new();

    for combinator in &schema.combinators {
        if combinator.typ == CombinatorType::Constructor {
            let name = match &combinator.data_type {
                DataType::Defined(name) => name.clone(),
                _ => unreachable!(),
            };
            map.entry(name)
                .or_insert_with(Vec::new)
                .push(combinator)
        }
    }

    map.into_iter()
        .map(|(name, constructors)| Type { name, constructors })
        .collect()
}

fn collect_combinators(schema: &Schema, typ: CombinatorType) -> Vec<&Combinator> {
    schema.combinators
        .iter()
        .filter(|c| c.typ == typ)
        .collect()
}

fn group_types(items: Vec<Type>) -> Namespace<Namespace<Type>> {
    let mut map = HashMap::new();

    for item in items {
        map.entry(item.name.namespace.clone())
            .or_insert_with(Vec::new)
            .push(item);
    }

    let items = map.into_iter()
        .map(|(name, items)| Namespace { name, items })
        .collect();

    Namespace {
        name: Some(TYPES_NAMESPACE.to_string()),
        items,
    }
}

fn group_combinators(items: Vec<&Combinator>, typ: CombinatorType) -> Namespace<Namespace<&Combinator>> {
    let mut map = HashMap::new();

    for item in items {
        map.entry(item.name.namespace.clone())
            .or_insert_with(Vec::new)
            .push(item);
    }

    let items = map.into_iter()
        .map(|(name, items)| Namespace { name, items })
        .collect();

    Namespace {
        name: Some(match typ {
            CombinatorType::Constructor => CONSTRUCTORS_NAMESPACE.to_string(),
            CombinatorType::Function => FUNCTIONS_NAMESPACE.to_string(),
        }),
        items,
    }
}

fn map_namespaces(schema: &Schema, types: &[Type]) -> HashMap<Name, String> {
    let mut map = HashMap::new();

    for combinator in &schema.combinators {
        map.insert(combinator.name.clone(), match combinator.typ {
            CombinatorType::Constructor => CONSTRUCTORS_NAMESPACE.to_string(),
            CombinatorType::Function => FUNCTIONS_NAMESPACE.to_string(),
        });
    }

    for item in types {
        map.insert(item.name.clone(), TYPES_NAMESPACE.to_string());
    }

    map
}

fn map_types(types: &[Type]) -> HashMap<Name, Name> {
    types.iter()
        .flat_map(|t| {
            t.constructors.iter()
                .map(|c| (c.name.clone(), t.name.clone()))
        })
        .collect()
}
