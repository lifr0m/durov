use crate::{CombinatorType, DataType, Name, Schema};
use std::collections::HashSet;

pub fn check_schema(schema: &Schema) {
    let available_names = collect_available_names(schema);
    check_combinators(schema, &available_names);
    check_fields(schema, &available_names);
}

fn collect_available_names(schema: &Schema) -> HashSet<Name> {
    schema.combinators.iter()
        .flat_map(|c| {
            let mut names = vec![c.name.clone()];
            if c.typ == CombinatorType::Constructor {
                names.push(match &c.data_type {
                    DataType::Defined(name) => name.clone(),
                    _ => unreachable!(),
                })
            }
            names
        })
        .collect()
}

fn check_combinators(schema: &Schema, available_names: &HashSet<Name>) {
    for combinator in &schema.combinators {
        check_data_type(&combinator.data_type, available_names, &[]);
    }
}

fn check_fields(schema: &Schema, available_names: &HashSet<Name>) {
    for combinator in &schema.combinators {
        let mut condition_fields = Vec::new();
        for field in &combinator.fields {
            if field.typ == DataType::Condition {
                condition_fields.push(field.name.clone());
                continue;
            }
            check_data_type(&field.typ, available_names, &condition_fields);
        }
    }
}

fn check_data_type(typ: &DataType, available_names: &HashSet<Name>, condition_fields: &[String]) {
    match typ {
        DataType::Vector(typ) => check_data_type(typ, available_names, condition_fields),
        DataType::BareVector(typ) => check_data_type(typ, available_names, condition_fields),
        DataType::Defined(name) => assert!(available_names.contains(name)),
        DataType::Conditional { field, typ, .. } => {
            check_data_type(typ, available_names, condition_fields);
            assert!(condition_fields.contains(field));
        }
        DataType::Boxed(typ) => check_data_type(typ, available_names, condition_fields),
        _ => (),
    }
}
