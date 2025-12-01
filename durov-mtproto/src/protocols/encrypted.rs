pub mod object;

use crate::crypto;
use crate::log::debug_bytes;
use crate::protocols::checkers::{check_auth_key_id, check_msg_id, check_msg_len};
use crate::protocols::constants::{GZIP_PACKED_ID, MSG_CONTAINER_ID, RPC_RESULT_ID};
use crate::protocols::plain::Plain;
use crate::protocols::serialize::serialize_len_first;
use crate::protocols::time::{get_msg_id, get_now};
use crate::protocols::Error;
use durov_tl_types::buffer::Buffer;
use durov_tl_types::cursor::Cursor;
use durov_tl_types::deserialize::Deserialize;
use durov_tl_types::serialize::Serialize;
use durov_tl_types::{deserialize, multiple_deserialize_object, Object};
use flate2::bufread::GzDecoder;
use object::{InObject, OutObject};
use std::collections::BTreeSet;
use std::io::Read;

pub struct RpcResult {
    pub req_msg_id: i64,
    pub result: Object,
}

pub struct Encrypted {
    time_diff: f64,
    msg_id_history: BTreeSet<i64>,
    auth_key: [u8; 256],
    auth_key_id: i64,
    salt: i64,
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
            salt: 0,
            session_id: rand::random(),
            msg_seq: 0,
        }
    }

    pub fn from_plain(protocol: Plain, auth_key: [u8; 256], salt: i64) -> Self {
        Self {
            time_diff: protocol.time_diff,
            msg_id_history: protocol.msg_id_history,
            auth_key,
            auth_key_id: crypto::compute_auth_key_id(&auth_key),
            salt,
            session_id: rand::random(),
            msg_seq: 0,
        }
    }

    pub fn is_ready(&self) -> bool {
        self.salt != 0
    }

    pub fn set_server_time(&mut self, server_time: f64) {
        self.time_diff = get_now() - server_time;
    }

    pub fn set_salt(&mut self, salt: i64) {
        self.salt = salt;
    }
}

impl Encrypted {
    const ENCRYPTED: usize = 8 + 16;
    const DECRYPTED: usize = Self::ENCRYPTED + 8 + 8;
    const MESSAGE: usize = Self::DECRYPTED + 8 + 4 + 4;

    pub fn pack(&mut self, buf: &mut Buffer, objects: &[InObject]) -> Vec<i64> {
        let message_ids = match objects.len() {
            1 => self.pack_one_object(buf, &objects[0]),
            _ => self.pack_many_objects(buf, objects),
        };

        buf.extend_front(&self.session_id.to_le_bytes());
        buf.extend_front(&self.salt.to_le_bytes());

        debug_bytes("protocol [encrypted] (pack) [decrypted]", buf);

        let pad_len = crypto::calc_pad_len(buf.len(), 16);
        let pad_len = if pad_len < 12 { pad_len + 16 } else { pad_len };
        crypto::extend_random(buf, pad_len);

        let msg_key = crypto::compute_msg_key(
            &self.auth_key,
            crypto::Direction::ClientToServer,
            buf,
        );
        let (aes_key, aes_iv) = crypto::compute_aes_key_iv(
            &self.auth_key,
            &msg_key,
            crypto::Direction::ClientToServer,
        );
        crypto::aes256_ige_encrypt(buf, aes_key, aes_iv);

        buf.extend_front(&msg_key);
        buf.extend_front(&self.auth_key_id.to_le_bytes());

        debug_bytes("protocol [encrypted] (pack) [encrypted]", buf);

        message_ids
    }

    fn pack_one_object(&mut self, buf: &mut Buffer, object: &InObject) -> Vec<i64> {
        let msg_id = self.pack_object(buf, object);

        vec![msg_id]
    }

    fn pack_many_objects(&mut self, buf: &mut Buffer, objects: &[InObject]) -> Vec<i64> {
        MSG_CONTAINER_ID.serialize(buf);
        (objects.len() as i32).serialize(buf);

        let mut message_ids = Vec::new();
        for obj in objects {
            let msg_id = self.pack_object(buf, obj);
            message_ids.push(msg_id);
        }

        let len = buf.len() as i32;
        buf.extend_front(&len.to_le_bytes());

        let seq = self.next_msg_seq(false);
        buf.extend_front(&seq.to_le_bytes());

        let msg_id = get_msg_id(self.time_diff);
        buf.extend_front(&msg_id.to_le_bytes());
        message_ids.push(msg_id);

        message_ids
    }

    fn pack_object(&mut self, buf: &mut Buffer, object: &InObject) -> i64 {
        let msg_id = get_msg_id(self.time_diff);
        msg_id.serialize(buf);
        self.next_msg_seq(is_content(object.id)).serialize(buf);
        serialize_len_first(buf, &object.body);
        msg_id
    }

    fn next_msg_seq(&mut self, content: bool) -> i32 {
        if content {
            let msg_seq = self.msg_seq * 2 + 1;
            self.msg_seq += 1;
            msg_seq
        } else {
            self.msg_seq * 2
        }
    }

    pub fn unpack(&mut self, buf: &mut Buffer) -> Result<Vec<OutObject>, Error> {
        debug_bytes("protocol [encrypted] (unpack) [encrypted]", buf);

        if buf.len() < Self::ENCRYPTED {
            return Err(Error::MissingBytes);
        }

        let auth_key_id = i64::from_le_bytes(buf.array(0));

        check_auth_key_id(self.auth_key_id, auth_key_id)?;

        let msg_key = buf.array(8);

        let (aes_key, aes_iv) = crypto::compute_aes_key_iv(
            &self.auth_key,
            &msg_key,
            crypto::Direction::ServerToClient,
        );

        crypto::aes256_ige_decrypt(&mut buf[Self::ENCRYPTED..], aes_key, aes_iv);

        debug_bytes("protocol [encrypted] (unpack) [decrypted]", &buf[Self::ENCRYPTED..]);

        let calc_msg_key = crypto::compute_msg_key(
            &self.auth_key,
            crypto::Direction::ServerToClient,
            &buf[Self::ENCRYPTED..],
        );
        if calc_msg_key != msg_key {
            return Err(Error::MsgKeyMismatch {
                expected: calc_msg_key,
                received: msg_key,
            });
        }

        if buf.len() < Self::DECRYPTED {
            return Err(Error::MissingBytes);
        }

        let session_id = i64::from_le_bytes(buf.array(Self::ENCRYPTED + 8));

        if session_id != self.session_id {
            return Err(Error::SessionIdMismatch {
                expected: self.session_id,
                received: session_id,
            });
        }

        if buf.len() < Self::MESSAGE {
            return Err(Error::MissingBytes);
        }

        let len = i32::from_le_bytes(buf.array(Self::DECRYPTED + 12));

        check_msg_len(len, buf.len() - Self::MESSAGE)?;

        let pad_len = buf.len() - Self::MESSAGE - len as usize;

        if !(12..=1024).contains(&pad_len) {
            return Err(Error::InvalidPaddingLength(pad_len));
        }

        let mut cur = Cursor::new(&buf[Self::DECRYPTED..]);
        self.unpack_message(&mut cur)
    }

    fn unpack_message(&mut self, src: &mut Cursor) -> Result<Vec<OutObject>, Error> {
        let msg_id = i64::deserialize(src)?;
        let _seq = i32::deserialize(src)?;
        let _len = i32::deserialize(src)?;

        let id = i32::deserialize(src)?;

        match check_msg_id(self.time_diff, &mut self.msg_id_history, msg_id, Some(id)) {
            Ok(()) => (),
            Err(Error::IgnoreThisMessage) => {
                log::warn!("ignoring message: {msg_id}");
                return Ok(Vec::new());
            }
            Err(err) => return Err(err),
        }

        Ok(match id {
            MSG_CONTAINER_ID => {
                let len = i32::deserialize(src)?;
                let mut objects = Vec::new();
                for _ in 0..len {
                    let chunk = self.unpack_message(src)?;
                    objects.extend(chunk);
                }
                objects
            }
            _ => {
                src.seek(-4);
                let object = deserialize_object(src)?;
                vec![OutObject::new(msg_id, object)]
            }
        })
    }
}

fn is_content(id: i32) -> bool {
    durov_tl_types::schemas::api::ALL_IDS.contains(&id)
}

fn deserialize_object(src: &mut Cursor) -> Result<Object, deserialize::Error> {
    let id = i32::deserialize(src)?;
    Ok(match id {
        GZIP_PACKED_ID => {
            let packed_data = Vec::<u8>::deserialize(src)?;
            let mut decoder = GzDecoder::new(&*packed_data);
            let mut data = Vec::new();
            decoder.read_to_end(&mut data)
                .map_err(deserialize::Error::GzipDecode)?;
            let mut cur = Cursor::new(&data);
            deserialize_object(&mut cur)?
        }
        RPC_RESULT_ID => {
            let req_msg_id = i64::deserialize(src)?;
            let result = deserialize_object(src)?;
            let object = RpcResult { req_msg_id, result };
            Object {
                id: RPC_RESULT_ID,
                body: Box::new(object),
            }
        }
        _ => {
            src.seek(-4);
            multiple_deserialize_object(src, &[
                durov_tl_types::schemas::mtproto::deserialize_object,
                durov_tl_types::schemas::api::deserialize_object,
            ])?
        }
    })
}
