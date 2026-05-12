pub mod item;

use crate::encrypted::timed::Timed;
use item::Provided;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::time;

type OnReturn<T> = fn(&mut T);

pub struct Pool<T> {
    pool: Arc<Mutex<Vec<Timed<T>>>>,
    on_return: OnReturn<T>,
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
    pub fn new(on_return: OnReturn<T>, timeout: Duration) -> Self {
        let pool = Arc::new(Mutex::new(Vec::new()));
        tokio::spawn(remove_expired_loop(pool.clone(), timeout));
        Self { pool, on_return }
    }
}

impl<T: Default> Pool<T> {
    pub fn provide(&self) -> Provided<T> {
        let item = self.pool.lock().unwrap()
            .pop()
            .map(|item| item.value)
            .unwrap_or_default();
        Provided::new(self.clone(), item)
    }
}

impl<T> Pool<T> {
    fn put(&self, mut item: T) {
        (self.on_return)(&mut item);
        self.pool.lock().unwrap()
            .push(Timed::new(item));
    }
}

async fn remove_expired_loop<T>(pool: Arc<Mutex<Vec<Timed<T>>>>, timeout: Duration) {
    loop {
        pool.lock().unwrap()
            .retain(|item| !item.expired(timeout));
        time::sleep(Duration::from_secs(1)).await;
    }
}
