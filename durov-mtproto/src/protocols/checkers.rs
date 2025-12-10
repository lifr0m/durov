use crate::protocols::time::{get_now, parse_msg_id};
use crate::protocols::Error;
use std::collections::BTreeSet;

const MSG_ID_HISTORY_SIZE: usize = 4 * (1 + 1024);

const SKIP_TIME_CHECK: &[i32] = &[
    0x9ec20908_u32 as i32, // new_session_created
    0xa7eff811_u32 as i32, // bad_msg_notification
    0xedab447b_u32 as i32, // bad_server_salt
];

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
        return Err(Error::InvalidLength(len));
    }

    let len = len as usize;

    if len > max_len {
        return Err(Error::LengthTooBig {
            expected: max_len,
            received: len,
        });
    }

    Ok(())
}

pub fn check_msg_id(
    time_diff: f64,
    history: &mut BTreeSet<i64>,
    msg_id: i64,
    id: Option<i32>,
) -> Result<(), Error> {
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

    if history.len() >= MSG_ID_HISTORY_SIZE && msg_id < *history.first().unwrap() {
        return Err(Error::IgnoreThisMessage);
    }
    if history.contains(&msg_id) {
        return Err(Error::IgnoreThisMessage);
    }

    if history.len() >= MSG_ID_HISTORY_SIZE {
        history.pop_first();
    }
    history.insert(msg_id);

    Ok(())
}
