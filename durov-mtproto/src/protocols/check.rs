use crate::protocols::time::{get_now, parse_msg_id};
use crate::protocols::Error;
use durov_tl_types::Identify;
use std::collections::BTreeSet;
use std::thread;

const SKIP_TIME_CHECK: &[i32] = &[
    durov_tl_types::schemas::mtproto::types::NewSessionCreated::ID,
    durov_tl_types::schemas::mtproto::types::BadMsgNotification::ID,
    durov_tl_types::schemas::mtproto::types::BadServerSalt::ID,
];

pub fn msg_id_history_size() -> usize {
    let cpu_count = thread::available_parallelism().unwrap().get();
    // 4 - network margin
    // cpu_count - parallelism on protocol
    // 1 - msg container
    // 1024 - max container size
    4 * cpu_count * (1 + 1024)
}

pub fn check_auth_key_id(auth_key_id: i64, packet_auth_key_id: i64) -> Result<(), Error> {
    if auth_key_id == packet_auth_key_id {
        Ok(())
    } else {
        Err(Error::AuthKeyIdMismatch {
            expected: auth_key_id,
            received: packet_auth_key_id,
        })
    }
}

pub fn check_msg_len(len: i32, max_len: usize) -> Result<(), Error> {
    if len % 4 != 0 || len < 0 {
        return Err(Error::InvalidMsgLength(len));
    }

    let len = len as usize;

    if len > max_len {
        return Err(Error::MsgLengthTooBig {
            expected: max_len,
            received: len,
        });
    }

    Ok(())
}

pub fn check_msg_id(time_diff: f64, history: &mut BTreeSet<i64>, msg_id: i64, id: Option<i32>)
    -> Result<(), Error>
{
    ensure_time_sync(time_diff, msg_id, id)?;
    ensure_msg_id(history, msg_id)?;

    Ok(())
}

fn ensure_time_sync(diff: f64, msg_id: i64, id: Option<i32>) -> Result<(), Error> {
    if diff == 0.0 {
        return Ok(());
    }
    if let Some(id) = id && SKIP_TIME_CHECK.contains(&id) {
        return Ok(());
    }

    let expected_now = get_now() + diff;
    let received_now = parse_msg_id(msg_id);
    let gap = received_now - expected_now;

    if !(-300.0..30.0).contains(&gap) {
        return Err(Error::IgnoreThisMessage);
    }

    Ok(())
}

fn ensure_msg_id(history: &mut BTreeSet<i64>, msg_id: i64) -> Result<(), Error> {
    if msg_id % 2 != 1 {
        return Err(Error::InvalidMsgId(msg_id));
    }

    if history.len() >= msg_id_history_size() && msg_id < *history.first().unwrap() {
        return Err(Error::IgnoreThisMessage);
    }
    if history.contains(&msg_id) {
        return Err(Error::IgnoreThisMessage);
    }

    if history.len() >= msg_id_history_size() {
        history.pop_first();
    }
    history.insert(msg_id);

    Ok(())
}
