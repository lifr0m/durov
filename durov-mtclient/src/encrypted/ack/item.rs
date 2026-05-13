use tokio::time;

pub struct Item {
    pub msg_id: i64,
    pub created: time::Instant,
}

impl Item {
    pub fn new(msg_id: i64) -> Self {
        Self {
            msg_id,
            created: time::Instant::now(),
        }
    }
}
