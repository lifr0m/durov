use std::time::Instant;

pub struct Item {
    pub msg_id: i64,
    pub created_at: Instant,
}

impl Item {
    pub fn new(msg_id: i64) -> Self {
        Self {
            msg_id,
            created_at: Instant::now(),
        }
    }
}
