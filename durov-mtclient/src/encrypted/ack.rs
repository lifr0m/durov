mod item;

use item::Item;
use std::cmp::min;
use std::collections::VecDeque;
use std::time::Duration;
use tokio::time;

const TIMEOUT: Duration = Duration::from_secs(30);
const THRESHOLD: usize = 16;

pub struct Ack {
    pending: VecDeque<Item>,
}

impl Ack {
    pub fn new() -> Self {
        Self {
            pending: VecDeque::new(),
        }
    }

    pub fn add(&mut self, msg_id: i64) {
        let item = Item::new(msg_id);
        self.pending.push_back(item);
    }

    pub fn next_batch(&mut self) -> Vec<i64> {
        let count = min(self.pending.len(), 8192);
        self.pending.drain(..count)
            .map(|item| item.msg_id)
            .collect()
    }

    pub async fn wait(&mut self) {
        if self.pending.len() > THRESHOLD {
            return;
        }
        let deadline = self.pending[0].created + TIMEOUT;
        let duration = deadline - time::Instant::now();
        time::sleep(duration).await;
    }

    pub fn condition(&self) -> bool {
        !self.pending.is_empty()
    }
}
