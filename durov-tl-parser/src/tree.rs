#[derive(Debug)]
pub struct Node<T> {
    data: T,
    children: Vec<Node<T>>,
}

impl<T> Node<T> {
    pub fn new(data: T) -> Self {
        Self {
            data,
            children: Vec::new(),
        }
    }

    pub fn data(&self) -> &T {
        &self.data
    }

    pub fn children(&self) -> &Vec<Node<T>> {
        &self.children
    }

    pub fn children_mut(&mut self) -> &mut Vec<Node<T>> {
        &mut self.children
    }

    pub fn add_child(&mut self, child: Node<T>) {
        self.children.push(child);
    }
}
