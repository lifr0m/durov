use durov_tl_parser::Name;
use std::collections::HashMap;

pub struct Context {
    pub nested: usize,
    pub namespaces: HashMap<Name, String>,
}

impl Context {
    pub fn new(namespaces: HashMap<Name, String>) -> Self {
        Self {
            nested: 0,
            namespaces,
        }
    }
}
