use crate::encrypted::pool::Pool;
use std::ops::{Deref, DerefMut};

pub struct Provided<T> {
    pool: Pool<T>,
    item: Option<T>,
}

impl<T> Provided<T> {
    pub fn new(pool: Pool<T>, item: T) -> Self {
        Self {
            pool,
            item: Some(item),
        }
    }
}

impl<T> Drop for Provided<T> {
    fn drop(&mut self) {
        let item = self.item.take()
            .unwrap();
        self.pool.put(item);
    }
}

impl<T> Deref for Provided<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.item.as_ref()
            .unwrap()
    }
}

impl<T> DerefMut for Provided<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.item.as_mut()
            .unwrap()
    }
}
