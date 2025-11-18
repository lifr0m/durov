use crate::writer::Writer;
use durov_tl_parser::Name;
use std::collections::HashMap;

pub struct Context {
    pub nested: usize,
    pub namespaces: HashMap<Name, String>,
}

impl Context {
    pub fn new(namespaces: HashMap<Name, String>) -> Self {
        Self { nested: 0, namespaces }
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
