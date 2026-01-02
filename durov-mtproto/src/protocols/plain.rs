use crate::log::debug_bytes;
use crate::protocols::check::{check_auth_key_id, check_msg_id, check_msg_len};
use crate::protocols::serde::serialize_len_first;
use crate::protocols::time::{get_msg_id, get_now};
use crate::protocols::Error;
use durov_tl_types::buffer::Buffer;
use durov_tl_types::deserialize::Deserialize;
use durov_tl_types::serialize::Serialize;
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
        self.time_diff = server_time - get_now();
    }
}

impl Plain {
    const FULL: usize = 8 + 8 + 4;

    pub fn pack<T: Serialize>(&mut self, buf: &mut Buffer, object: T) {
        0_i64.serialize(buf);
        get_msg_id(self.time_diff).serialize(buf);
        serialize_len_first(buf, |buf| object.serialize(buf));
        debug_bytes("protocol [plain] (pack)", buf);
    }

    pub fn unpack<T: Deserialize>(&mut self, buf: &Buffer) -> Result<T, Error> {
        debug_bytes("protocol [plain] (unpack)", buf);

        if buf.len() < Self::FULL {
            return Err(Error::MissingBytes);
        }

        let auth_key_id = i64::from_le_bytes(buf.array(0));

        check_auth_key_id(0, auth_key_id)?;

        let msg_id = i64::from_le_bytes(buf.array(8));

        check_msg_id(self.time_diff, &mut self.msg_id_history, msg_id, None)?;

        let len = i32::from_le_bytes(buf.array(16));

        check_msg_len(len, buf.len() - Self::FULL)?;

        let object = T::from_bytes(&buf[Self::FULL..])?;

        Ok(object)
    }
}
