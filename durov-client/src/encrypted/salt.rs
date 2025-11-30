mod item;

use durov_mtproto::protocols::time::get_now;
use item::Item;
use std::collections::VecDeque;
use std::time::Duration;

pub struct FutureSalts {
    list: VecDeque<Item>,
    pub asked: f64,
}

impl FutureSalts {
    pub fn new() -> Self {
        Self {
            list: VecDeque::new(),
            asked: 0.0,
        }
    }

    pub fn add(&mut self, salt: i64, since: f64) {
        let item = Item { salt, since };
        self.list.push_back(item);
    }

    pub fn pop(&mut self) -> i64 {
        self.list.pop_front()
            .unwrap()
            .salt
    }

    pub async fn select(&mut self) {
        let deadline = if self.can_get() {
            self.list[0].since
        } else {
            self.asked + 30.0
        };
        let duration = deadline - get_now(0.0);
        let duration = if duration < 0.0 { 0.0 } else { duration };
        let duration = Duration::from_secs_f64(duration);
        tokio::time::sleep(duration).await
    }

    pub fn can_get(&self) -> bool {
        !self.list.is_empty()
    }
}
