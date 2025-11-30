use crate::writer::Writer;
use durov_tl_parser::Name;
use std::collections::HashMap;

pub struct Context {
    pub nested: usize,
    pub namespaces: HashMap<Name, String>,
    pub combinator_type_map: HashMap<Name, Name>,
}

impl Context {
    pub fn new(namespaces: HashMap<Name, String>, combinator_type_map: HashMap<Name, Name>) -> Self {
        Self {
            nested: 0,
            namespaces,
            combinator_type_map,
        }
    }
}

pub trait Write {
    fn write(&self, writer: &mut Writer, context: &mut Context);
}

impl<T: Write> Write for &T {
    fn write(&self, writer: &mut Writer, context: &mut Context) {
        (*self).write(writer, context);
    }
}
