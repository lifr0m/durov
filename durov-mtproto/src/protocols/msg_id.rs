pub const MSG_ID_HISTORY_SIZE: usize = 100;

pub fn get_now(diff: f64) -> f64 {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs_f64();
    now - diff
}

pub fn get_msg_id(diff: f64) -> i64 {
    let now = get_now(diff);
    let msg_id = now * 2_f64.powi(32);
    let msg_id = msg_id as i64;
    msg_id - msg_id % 4
}

pub fn parse_msg_id(msg_id: i64) -> f64 {
    let msg_id = msg_id as f64;
    msg_id / 2_f64.powi(32)
}
