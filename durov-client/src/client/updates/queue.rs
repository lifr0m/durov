use std::collections::BTreeMap;
use tokio::time;

pub struct Queue<T> {
    items: BTreeMap<(i32, i32), Vec<T>>,
    pub gap_since: Option<time::Instant>,
}

impl<T> Default for Queue<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> Queue<T> {
    pub fn new() -> Self {
        Self {
            items: BTreeMap::new(),
            gap_since: None,
        }
    }

    pub fn put(&mut self, item: T, pts: i32, count: i32) {
        self.items.entry((pts, -count))
            .or_default()
            .push(item);
    }

    pub fn peek(&self) -> Option<(i32, i32)> {
        self.items.first_key_value()
            .map(|(&(pts, count), _)| (pts, -count))
    }

    pub fn take(&mut self) -> Vec<T> {
        self.items.pop_first()
            .map(|(_, list)| list)
            .unwrap()
    }
}
