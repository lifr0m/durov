use super::{Error, Protocol};
use crate::crypto;
use crate::protocols::plain::Plain;
use std::collections::BTreeSet;

enum MsgRelated {
    Content,
    NonContent,
}

pub struct Encrypted {
    time_diff: f64,
    msg_id_history: BTreeSet<i64>,
    auth_key: [u8; 256],
    auth_key_id: i64,
    salt: [u8; 8],
    session_id: i64,
    msg_seq: i32,
}

impl Encrypted {
    pub fn new(auth_key: [u8; 256]) -> Self {
        Self {
            time_diff: 0.0,
            msg_id_history: BTreeSet::new(),
            auth_key,
            auth_key_id: crypto::compute_auth_key_id(&auth_key),
            salt: rand::random(),
            session_id: rand::random(),
            msg_seq: 0,
        }
    }

    pub fn from_plain(protocol: Plain, auth_key: [u8; 256], salt: [u8; 8]) -> Self {
        Self {
            time_diff: protocol.time_diff,
            msg_id_history: protocol.msg_id_history,
            auth_key,
            auth_key_id: crypto::compute_auth_key_aux_id(&auth_key),
            salt,
            session_id: rand::random(),
            msg_seq: 0,
        }
    }

    pub fn auth_key(&self) -> &[u8] {
        &self.auth_key
    }
}

impl Protocol for Encrypted {
    fn pack(&mut self, message: &[u8]) -> Result<Vec<u8>, Error> {
        todo!()
    }

    fn unpack(&mut self, data: &[u8]) -> Result<Vec<u8>, Error> {
        todo!()
    }
}

impl Encrypted {
    const PLAIN_OVERHEAD: usize = 8 + 8 + 8 + 4 + 4;

    fn next_msg_seq(&mut self, related: MsgRelated) -> i32 {
        match related {
            MsgRelated::Content => {
                let msg_seq = self.msg_seq * 2 + 1;
                self.msg_seq += 1;
                msg_seq
            }
            MsgRelated::NonContent => self.msg_seq * 2,
        }
    }
}
