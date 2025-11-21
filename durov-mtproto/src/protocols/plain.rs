use super::msg_id::{get_msg_id, get_now, parse_msg_id, MSG_ID_HISTORY_SIZE};
use super::{Error, Protocol};
use crate::log::debug_bytes;
use std::collections::BTreeSet;

pub struct Plain {
    pub(super) time_diff: f64,
    pub(super) msg_id_history: BTreeSet<i64>,
}

impl Default for Plain {
    fn default() -> Self {
        Self::new()
    }
}

impl Plain {
    pub fn new() -> Self {
        Self {
            time_diff: 0.0,
            msg_id_history: BTreeSet::new(),
        }
    }

    pub fn set_server_time(&mut self, server_time: f64) {
        self.time_diff = get_now(0.0) - server_time;
    }
}

impl Protocol for Plain {
    fn pack(&mut self, message: &[u8]) -> Result<Vec<u8>, Error> {
        let mut data = Vec::new();
        data.extend(0i64.to_le_bytes());
        data.extend(get_msg_id(self.time_diff).to_le_bytes());
        data.extend((message.len() as i32).to_le_bytes());
        data.extend(message);
        debug_bytes("protocol [plain] (pack)", [&data]);
        Ok(data)
    }

    fn unpack(&mut self, data: &[u8]) -> Result<Vec<u8>, Error> {
        debug_bytes("protocol [plain] (unpack)", [data]);

        if data.len() < Self::OVERHEAD {
            return Err(Error::DataTooShort {
                expected: Self::OVERHEAD,
                received: data.len(),
            });
        }

        let auth_key_id = i64::from_le_bytes(data[0..8].try_into().unwrap());

        if auth_key_id != 0 {
            return Err(Error::WrongAuthKeyId {
                expected: 0,
                received: auth_key_id,
            });
        }

        let message_id = i64::from_le_bytes(data[8..16].try_into().unwrap());

        if self.msg_id_history.len() >= MSG_ID_HISTORY_SIZE
            && self.msg_id_history.iter().all(|&msg_id| message_id < msg_id)
            || self.msg_id_history.contains(&message_id)
        {
            return Err(Error::IgnoreThisMessage);
        }
        if self.msg_id_history.len() >= MSG_ID_HISTORY_SIZE {
            self.msg_id_history.pop_first();
        }
        self.msg_id_history.insert(message_id);

        // todo: ignore this check for certain messages
        if self.time_diff != 0.0 {
            let now = get_now(self.time_diff);
            let server_now = parse_msg_id(message_id);
            if server_now - now > 30.0 || now - server_now > 300.0 {
                return Err(Error::IgnoreThisMessage);
            }
        }

        let message_len = i32::from_le_bytes(data[16..20].try_into().unwrap()) as usize;

        if Self::OVERHEAD + message_len != data.len() {
            return Err(Error::WrongMessageLength {
                expected: message_len,
                received: data.len() - Self::OVERHEAD,
            });
        }

        let message = data[Self::OVERHEAD..].to_vec();

        Ok(message)
    }
}

impl Plain {
    const OVERHEAD: usize = 8 + 8 + 4;
}
