pub mod writer;
mod write;
mod namespace;
mod type_info;
mod combinator;
mod data_type;
mod name;
mod context;

use context::Context;
use durov_tl_parser::{Combinator, CombinatorType, DataType, Name, Schema};
use namespace::Namespace;
use std::collections::HashMap;
use type_info::Type;
use write::Write;
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
    let mut context = Context::new(map_namespaces(schema, &types));
    let types = group_types(types);
    types.write(&mut writer, &mut context);

    let constructors = collect_combinators(schema, CombinatorType::Constructor);
    let constructors = group_combinators(constructors, CombinatorType::Constructor);
    constructors.write(&mut writer, &mut context);

    let functions = collect_combinators(schema, CombinatorType::Function);
    let functions = group_combinators(functions, CombinatorType::Function);
    functions.write(&mut writer, &mut context);

    writer.destruct()
}

fn write_layer(writer: &mut Writer, layer: i32) {
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
