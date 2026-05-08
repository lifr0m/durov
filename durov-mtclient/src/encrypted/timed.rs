use std::time::Duration;
use tokio::time;

pub struct Timed<T> {
    pub value: T,
    created: time::Instant,
}

impl<T> Timed<T> {
    pub fn new(value: T) -> Self {
        Self {
            value,
            created: time::Instant::now(),
        }
    }

    pub fn expired(&self, timeout: Duration) -> bool {
        self.created.elapsed() > timeout
    }
}
