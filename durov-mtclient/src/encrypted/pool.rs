pub mod item;

use crate::encrypted::timed::Timed;
use item::Provided;
use parking_lot::Mutex;
use std::sync::{Arc, Weak};
use std::time::Duration;
use tokio::time;

pub struct Pool<T> {
    pool: Arc<Mutex<Vec<Timed<T>>>>,
    on_return: fn(&mut T),
}

impl<T> Clone for Pool<T> {
    fn clone(&self) -> Self {
        Self {
            pool: Arc::clone(&self.pool),
            on_return: self.on_return,
        }
    }
}

impl<T: Send + 'static> Pool<T> {
    pub fn new(on_return: fn(&mut T), timeout: Duration) -> Self {
        let pool = Arc::new(Mutex::new(Vec::new()));
        tokio::spawn(remove_expired_loop(Arc::downgrade(&pool), timeout));
        Self { pool, on_return }
    }
}

impl<T: Default> Pool<T> {
    pub fn provide(&self) -> Provided<T> {
        let item = self.pool.lock().pop()
            .map(|item| item.value)
            .unwrap_or_default();
        Provided::new(self.clone(), item)
    }
}

impl<T> Pool<T> {
    fn put(&self, mut item: T) {
        (self.on_return)(&mut item);
        self.pool.lock().push(Timed::new(item));
    }
}

async fn remove_expired_loop<T>(pool: Weak<Mutex<Vec<Timed<T>>>>, timeout: Duration) {
    while let Some(pool) = pool.upgrade() {
        pool.lock().retain(|item| !item.expired(timeout));
        time::sleep(Duration::from_secs(1)).await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_pool() {
        let pool = Pool::new(|n| *n += 1, Duration::from_secs(1));
        assert_eq!(elements(&pool), []);

        let n = pool.provide();
        assert_eq!(*n, 0);
        assert_eq!(elements(&pool), []);

        drop(n);
        assert_eq!(elements(&pool), [1]);

        let n1 = pool.provide();
        let n2 = pool.provide();
        assert_eq!(*n1, 1);
        assert_eq!(*n2, 0);
        assert_eq!(elements(&pool), []);

        drop(n1);
        drop(n2);
        assert_eq!(elements(&pool), [2, 1]);

        time::sleep(Duration::from_secs(2)).await;

        assert_eq!(elements(&pool), []);
    }

    fn elements(pool: &Pool<i32>) -> Vec<i32> {
        pool.pool.lock().iter()
            .map(|item| item.value)
            .collect()
    }
}
