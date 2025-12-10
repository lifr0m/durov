use std::time::{SystemTime, UNIX_EPOCH};

pub fn get_now() -> f64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs_f64()
}

pub fn get_msg_id(diff: f64) -> i64 {
    let server_now = get_now() + diff;
    let msg_id = server_now * 2_f64.powi(32);
    let msg_id = msg_id as i64;
    msg_id - msg_id % 4
}

pub fn parse_msg_id(msg_id: i64) -> f64 {
    let msg_id = msg_id as f64;
    msg_id / 2_f64.powi(32)
}
