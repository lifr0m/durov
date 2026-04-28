mod recursion;
mod tree;

use crc_fast::CrcAlgorithm;

const IGNORED_COMBINATORS: &[&str] = &[
    "int ? = Int",
    "long ? = Long",
    "double ? = Double",
    "string ? = String",
    "vector {t:Type} # [ t ] = Vector t",
    "vector#1cb5c415 {t:Type} # [ t ] = Vector t",
    "int128 4*[ int ] = Int128",
    "int256 8*[ int ] = Int256",
    "boolFalse#bc799737 = Bool",
    "boolTrue#997275b5 = Bool",
    "true#3fedd339 = True",
];

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum CombinatorType {
    Constructor,
    Function,
}

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct Name {
    pub namespace: Option<String>,
    pub name: String,
}

#[derive(Debug, Eq, PartialEq)]
pub enum DataType {
    Int,
    Long,
    Double,
    String,
    Bytes,
    Bool,
    Vector(Box<DataType>),
    BareVector(Box<DataType>),
    Int128,
    Int256,
    Defined(Name),
    Polymorphic(String),
    PolymorphicFunction(String),
    PolymorphicFunctionResult(String),
    Condition,
    Conditional {
        field: String,
        bit: u8,
        typ: Box<DataType>,
    },
    ConditionalTrue {
        field: String,
        bit: u8,
    },
    Boxed(Box<DataType>),
}

#[derive(Debug)]
pub struct Field {
    pub name: String,
    pub typ: DataType,
}

#[derive(Debug)]
pub struct Combinator {
    pub typ: CombinatorType,
    pub name: Name,
    pub id: i32,
    pub data_type: DataType,
    pub fields: Vec<Field>,
}

impl Combinator {
    pub fn iter_data_types(&self) -> impl Iterator<Item = &DataType> {
        self.fields.iter()
            .map(|f| &f.typ)
            .chain([&self.data_type])
    }

    pub fn iter_data_types_mut(&mut self) -> impl Iterator<Item = &mut DataType> {
        self.fields.iter_mut()
            .map(|f| &mut f.typ)
            .chain([&mut self.data_type])
    }
}

#[derive(Debug)]
pub struct Schema {
    pub layer: Option<u16>,
    pub combinators: Vec<Combinator>,
}

pub fn parse_schema(input: &str) -> Schema {
    let mut layer = None;
    let mut combinator_type = CombinatorType::Constructor;
    let mut combinators = Vec::new();

    for line in input.split("\n") {
        println!("{line}");

        if line.is_empty() {
            continue;
        }
        if let Some(line) = line.strip_prefix("//") {
            if let Some(line) = line.strip_prefix(" LAYER ") {
                layer = Some(line.parse().unwrap());
            }
            continue;
        }

        match line {
            "---types---" => {
                combinator_type = CombinatorType::Constructor;
                continue;
            }
            "---functions---" => {
                combinator_type = CombinatorType::Function;
                continue;
            }
            _ => {}
        }
        let line = line.strip_suffix(";")
            .unwrap();

        if IGNORED_COMBINATORS.contains(&line) {
            continue;
        }

        combinators.push(replace_polymorphic(parse_combinator(line, combinator_type)));
    }

    let mut schema = Schema { layer, combinators };
    recursion::fix_recursion(&mut schema);
    schema
}

fn replace_polymorphic(mut combinator: Combinator) -> Combinator {
    if combinator.iter_data_types()
        .any(|typ| matches!(typ, DataType::PolymorphicFunction(_)))
    {
        for typ in combinator.iter_data_types_mut() {
            if let DataType::Polymorphic(name) = typ {
                *typ = DataType::PolymorphicFunctionResult(name.clone());
            }
        }
    }

    combinator
}

fn parse_combinator(line: &str, typ: CombinatorType) -> Combinator {
    let (name, id, line) = match line.split_once("#") {
        Some((name, line)) => {
            let name = parse_name(name);

            let (id, line) = line.split_once(" ")
                .unwrap();
            let id = u32::from_str_radix(id, 16)
                .map(|v| v as i32)
                .unwrap();

            (name, id, line)
        }
        None => {
            let id = calc_combinator_id(line);

            let (name, line) = line.split_once(" ")
                .unwrap();
            let name = parse_name(name);

            (name, id, line)
        }
    };

    let (line, poly_type) = parse_poly_type(line);

    let (line, data_type) = line.split_once("= ")
        .unwrap();
    let line = line.trim_end();

    let data_type = parse_data_type(data_type, poly_type);

    let fields = if line.is_empty() { Vec::new() } else {
        line.split(" ")
            .map(|line| parse_field(line, poly_type))
            .collect()
    };

    Combinator { typ, name, id, data_type, fields }
}

fn parse_poly_type(line: &str) -> (&str, Option<&str>) {
    let old_line = line;

    let (poly_type, line) = line.split_once(" ")
        .unwrap();

    if poly_type.starts_with("{") {
        let poly_type = poly_type.strip_prefix("{")
            .unwrap()
            .strip_suffix(":Type}")
            .unwrap();
        (line, Some(poly_type))
    } else {
        (old_line, None)
    }
}

fn parse_data_type(line: &str, poly_type: Option<&str>) -> DataType {
    match line {
        "int" => DataType::Int,
        "long" => DataType::Long,
        "double" => DataType::Double,
        "Bool" => DataType::Bool,
        "string" => DataType::String,
        "bytes" => DataType::Bytes,
        "int128" => DataType::Int128,
        "int256" => DataType::Int256,
        "#" => DataType::Condition,
        _ if Some(line) == poly_type => DataType::Polymorphic(line.to_string()),
        _ if line.starts_with("!") => DataType::PolymorphicFunction(
            line.strip_prefix("!")
                .unwrap()
                .to_string(),
        ),
        _ if line.starts_with("Vector<") => DataType::Vector(Box::new(parse_data_type(
            line.strip_prefix("Vector<")
                .unwrap()
                .strip_suffix(">")
                .unwrap(),
            poly_type,
        ))),
        _ if line.starts_with("vector<") => DataType::BareVector(Box::new(parse_data_type(
            line.strip_prefix("vector<")
                .unwrap()
                .strip_suffix(">")
                .unwrap(),
            poly_type,
        ))),
        _ if line.contains("?") => {
            let (condition, line) = line.split_once("?")
                .unwrap();

            let (field, bit) = condition.split_once(".")
                .unwrap();
            let field = field.to_string();
            let bit = bit.parse()
                .unwrap();

            if line == "true" {
                DataType::ConditionalTrue { field, bit }
            } else {
                let typ = Box::new(parse_data_type(line, poly_type));

                DataType::Conditional { field, bit, typ }
            }
        }
        _ => DataType::Defined(parse_name(line)),
    }
}

fn parse_field(line: &str, poly_type: Option<&str>) -> Field {
    let (name, line) = line.split_once(":")
        .unwrap();

    let name = name.to_string();
    let typ = parse_data_type(line, poly_type);

    Field { name, typ }
}

fn parse_name(line: &str) -> Name {
    match line.split_once(".") {
        Some((namespace, name)) => Name {
            namespace: Some(namespace.to_string()),
            name: name.to_string(),
        },
        None => Name {
            namespace: None,
            name: line.to_string(),
        },
    }
}

fn calc_combinator_id(line: &str) -> i32 {
    let line = line.replace("{", "")
        .replace("}", "");
    crc_fast::checksum(CrcAlgorithm::Crc32IsoHdlc, line.as_bytes()) as i32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calc_combinator_id() {
        assert_eq!(calc_combinator_id("vector {t:Type} # [ t ] = Vector t"), 0x1cb5c415_u32 as i32);
    }
}
