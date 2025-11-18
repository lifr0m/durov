use crate::tree::Node;
use crate::{CombinatorType, DataType, Name, Schema};
use std::collections::HashMap;

// 4 solves the problem but 5 is more robust
const DEPTH: usize = 5;

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
enum NameType {
    Type,
    Combinator,
}

struct Info {
    types: HashMap<Name, Vec<Name>>,
    combinators: HashMap<Name, Vec<(NameType, Name)>>,
}

impl Info {
    fn resolve(
        &self,
        name_type: NameType,
        name: &Name,
        parent: &mut Node<(NameType, Name)>,
        left: usize,
    ) {
        if left == 0 {
            return;
        }

        let children = match name_type {
            NameType::Type => self.resolve_type(name)
                .into_iter()
                .map(|name| (NameType::Combinator, name))
                .collect(),
            NameType::Combinator => self.resolve_combinator(name),
        };

        for child in children {
            let mut child = Node::new(child);
            self.resolve(child.data().0, &child.data().1.clone(), &mut child, left - 1);
            parent.add_child(child);
        }
    }

    fn resolve_type(&self, name: &Name) -> Vec<Name> {
        self.types.get(name)
            .unwrap()
            .clone()
    }

    fn resolve_combinator(&self, name: &Name) -> Vec<(NameType, Name)> {
        self.combinators.get(name)
            .unwrap()
            .clone()
    }
}

pub fn fix_recursion(schema: &mut Schema) {
    loop {
        let info = collect_info(schema);
        let mut clean = true;

        for name in info.combinators.keys() {
            let mut tree = Node::new((NameType::Combinator, name.clone()));
            info.resolve(tree.data().0, &tree.data().1.clone(), &mut tree, DEPTH);

            let target = tree.data().clone();
            retain_recursive(&mut tree, &target);

            if tree.children().is_empty() {
                continue;
            }
            clean = false;

            let combinator = schema.combinators.iter_mut()
                .find(|c| c.name == tree.data().1)
                .unwrap();
            for field in &mut combinator.fields {
                if let Some(name) = resolve_data_type(&field.typ) {
                    let eligible = tree.children()
                        .iter()
                        .any(|c| c.data().1 == name);
                    if eligible {
                        replace_data_type(&mut field.typ);
                        println!("replaced");
                    }
                }
            }
            // with break - 30 replacements, without - 35
            break;
        }
        println!("done");

        if clean {
            break;
        }
    }
}

fn retain_recursive(node: &mut Node<(NameType, Name)>, target: &(NameType, Name)) {
    node.children_mut()
        .retain_mut(|node| retain_recursive_inner(node, target));
}

fn retain_recursive_inner(node: &mut Node<(NameType, Name)>, target: &(NameType, Name)) -> bool {
    if node.data() == target {
        node.children_mut().clear();
        true
    } else if node.children().is_empty() {
        false
    } else {
        retain_recursive(node, target);
        !node.children().is_empty()
    }
}

fn collect_info(schema: &Schema) -> Info {
    let types = collect_types(schema);
    let combinators = collect_combinators(schema, &types);
    Info { types, combinators }
}

fn collect_types(schema: &Schema) -> HashMap<Name, Vec<Name>> {
    let mut map = HashMap::new();

    for combinator in &schema.combinators {
        if combinator.typ == CombinatorType::Constructor {
            let name = match &combinator.data_type {
                DataType::Defined(name) => name.clone(),
                _ => unreachable!(),
            };
            map.entry(name)
                .or_insert_with(Vec::new)
                .push(combinator.name.clone())
        }
    }

    map
}

fn collect_combinators(
    schema: &Schema,
    types: &HashMap<Name, Vec<Name>>,
) -> HashMap<Name, Vec<(NameType, Name)>> {
    let mut map = HashMap::new();

    for combinator in &schema.combinators {
        let mut vec = Vec::new();

        for field in &combinator.fields {
            if let Some(name) = resolve_data_type(&field.typ) {
                let name_type = if types.contains_key(&name) {
                    NameType::Type
                } else {
                    NameType::Combinator
                };
                vec.push((name_type, name));
            }
        }

        map.insert(combinator.name.clone(), vec);
    }

    map
}

fn resolve_data_type(typ: &DataType) -> Option<Name> {
    match typ {
        DataType::Vector(typ) => resolve_data_type(typ),
        DataType::BareVector(typ) => resolve_data_type(typ),
        DataType::Conditional { typ, .. } => resolve_data_type(typ),
        DataType::Defined(name) => Some(name.clone()),
        _ => None,
    }
}

fn replace_data_type(typ: &mut DataType) {
    match typ {
        DataType::Vector(typ) => replace_data_type(typ),
        DataType::BareVector(typ) => replace_data_type(typ),
        DataType::Conditional { typ, .. } => replace_data_type(typ),
        DataType::Defined(_) => {
            let mut actual = DataType::Int;
            std::mem::swap(&mut actual, typ);
            *typ = DataType::Boxed(Box::new(actual));
        }
        _ => unreachable!(),
    }
}
