pub mod object;
mod gzip;
mod unpack;

use crate::crypto;
use crate::protocols::check::{check_auth_key_id, check_msg_id, check_msg_len};
use crate::protocols::constants::*;
use crate::protocols::encrypted::gzip::gzip_encode;
use crate::protocols::encrypted::object::{deserialize_object, DeserializeObject, PackObject, UnpackObject};
use crate::protocols::encrypted::unpack::unpack_object;
use crate::protocols::plain::Plain;
use crate::protocols::serde::serialize_len_first;
use crate::protocols::time::{get_msg_id, get_now};
use crate::protocols::Error;
use crate::tracing::debug_bytes;
use durov_tl_types::buffer::Buffer;
use durov_tl_types::cursor::{Cursor, Seek};
use durov_tl_types::deserialize::Deserialize;
use durov_tl_types::schemas::mtproto as tl;
use durov_tl_types::serialize::Serialize;
use durov_tl_types::{deserialize, Identify};
use std::collections::BTreeSet;
use std::sync::{Arc, Mutex};

const SKIP_GZIP: &[i32] = &[
    durov_tl_types::schemas::api::functions::upload::SaveFilePart::ID,
    durov_tl_types::schemas::api::functions::upload::SaveBigFilePart::ID,
];

pub struct Packed {
    pub container_msg_id: Option<i64>,
    pub msg_ids: Vec<i64>,
}

#[derive(Copy, Clone)]
pub struct UnpackParams<'a> {
    pub list: &'static [DeserializeObject<'static>],
    pub resolve: &'a dyn Fn(i64) -> Option<DeserializeObject<'static>>,
}

pub struct Unpacked {
    pub msg_id: i64,
    pub object: UnpackObject,
}

pub struct RpcResult {
    pub req_msg_id: i64,
    pub result: UnpackObject,
}

#[derive(Clone)]
pub struct Encrypted {
    time_diff: Arc<Mutex<f64>>,
    msg_id_history: Arc<Mutex<BTreeSet<i64>>>,
    auth_key: [u8; 256],
    auth_key_id: i64,
    salt: Arc<Mutex<i64>>,
    session_id: i64,
    msg_seq: Arc<Mutex<i32>>,
    use_gzip: bool,
}

impl Encrypted {
    pub fn new(auth_key: [u8; 256], use_gzip: bool) -> Self {
        Self {
            time_diff: Arc::new(Mutex::new(0.0)),
            msg_id_history: Arc::new(Mutex::new(BTreeSet::new())),
            auth_key,
            auth_key_id: crypto::compute_auth_key_id(&auth_key),
            salt: Arc::new(Mutex::new(0)),
            session_id: rand::random(),
            msg_seq: Arc::new(Mutex::new(0)),
            use_gzip,
        }
    }

    pub fn from_plain(protocol: Plain, auth_key: [u8; 256], salt: i64, use_gzip: bool) -> Self {
        Self {
            time_diff: Arc::new(Mutex::new(protocol.time_diff)),
            msg_id_history: Arc::new(Mutex::new(protocol.msg_id_history)),
            auth_key,
            auth_key_id: crypto::compute_auth_key_id(&auth_key),
            salt: Arc::new(Mutex::new(salt)),
            session_id: rand::random(),
            msg_seq: Arc::new(Mutex::new(0)),
            use_gzip,
        }
    }

    pub fn set_server_time(&self, server_time: f64) {
        *self.time_diff.lock().unwrap() = server_time - get_now();
    }

    pub fn set_salt(&self, salt: i64) {
        *self.salt.lock().unwrap() = salt;
    }
}

macro_rules! skip_msg {
    ($src:expr, $end:expr, $($arg:tt)+) => {
        tracing::warn!($($arg)+);
        $src.seek(Seek::Position($end));
        return Ok(Vec::new());
    };
}

impl Encrypted {
    const ENCRYPTED: usize = 8 + 16;
    const DECRYPTED: usize = Self::ENCRYPTED + 8 + 8;
    const MESSAGE: usize = Self::DECRYPTED + 8 + 4 + 4;

    pub fn pack(&self, buf: &mut Buffer, objects: &[&PackObject]) -> Packed {
        let packed = match objects.len() {
            1 => self.pack_one_object(buf, objects[0]),
            _ => self.pack_many_objects(buf, objects),
        };

        buf.extend_front(&self.session_id.to_le_bytes());
        buf.extend_front(&self.salt.lock().unwrap().to_le_bytes());

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

        packed
    }

    fn pack_one_object(&self, buf: &mut Buffer, object: &PackObject) -> Packed {
        let msg_id = self.pack_object(buf, object);

        Packed {
            container_msg_id: None,
            msg_ids: vec![msg_id],
        }
    }

    fn pack_many_objects(&self, buf: &mut Buffer, objects: &[&PackObject]) -> Packed {
        MSG_CONTAINER_ID.serialize(buf);
        (objects.len() as i32).serialize(buf);

        let message_ids = objects.iter()
            .map(|obj| self.pack_object(buf, obj))
            .collect();

        let len = buf.len() as i32;
        buf.extend_front(&len.to_le_bytes());

        let seq = self.next_msg_seq(false);
        buf.extend_front(&seq.to_le_bytes());

        let msg_id = get_msg_id(*self.time_diff.lock().unwrap());
        buf.extend_front(&msg_id.to_le_bytes());

        Packed {
            container_msg_id: Some(msg_id),
            msg_ids: message_ids,
        }
    }

    fn pack_object(&self, buf: &mut Buffer, object: &PackObject) -> i64 {
        let msg_id = get_msg_id(*self.time_diff.lock().unwrap());
        msg_id.serialize(buf);

        let content = durov_tl_types::schemas::api::ALL_IDS.contains(&object.id());
        self.next_msg_seq(content).serialize(buf);

        serialize_len_first(buf, |buf| {
            if self.use_gzip && content && !SKIP_GZIP.contains(&object.id()) {
                let mut serialized = Buffer::new();
                object.serialize(&mut serialized);

                if serialized.len() > 255 {
                    let mut compressed = Buffer::new();
                    GZIP_PACKED_ID.serialize(&mut compressed);
                    let packed_data = gzip_encode(&serialized);
                    packed_data.serialize(&mut compressed);

                    if compressed.len() < serialized.len() {
                        buf.extend_back(&compressed);
                    } else {
                        buf.extend_back(&serialized);
                    }
                } else {
                    buf.extend_back(&serialized);
                }
            } else {
                object.serialize(buf);
            }
        });

        msg_id
    }

    fn next_msg_seq(&self, content: bool) -> i32 {
        let mut msg_seq = self.msg_seq.lock().unwrap();

        if content {
            let new_msg_seq = *msg_seq * 2 + 1;
            *msg_seq += 1;
            new_msg_seq
        } else {
            *msg_seq * 2
        }
    }

    pub fn unpack(&self, buf: &mut Buffer, params: UnpackParams)
        -> Result<Vec<Unpacked>, Error>
    {
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
        self.unpack_message(&mut cur, params)
    }

    fn unpack_message(&self, src: &mut Cursor, params: UnpackParams)
        -> Result<Vec<Unpacked>, Error>
    {
        let msg_id = i64::deserialize(src)?;
        let _seq = i32::deserialize(src)?;
        let len = i32::deserialize(src)? as usize;

        let end = src.tell() + len;

        let id = i32::deserialize(src)?;

        match check_msg_id(
            *self.time_diff.lock().unwrap(),
            &mut self.msg_id_history.lock().unwrap(),
            msg_id,
            Some(id),
        ) {
            Ok(()) => {}
            Err(Error::IgnoreThisMessage) => {
                skip_msg!(src, end, "ignoring message: {msg_id}");
            }
            Err(err) => return Err(err),
        }

        match id {
            MSG_CONTAINER_ID => {
                let len = i32::deserialize(src)? as usize;

                let mut list = Vec::new();
                for _ in 0..len {
                    let chunk = self.unpack_message(src, params)?;
                    list.extend(chunk);
                }
                Ok(list)
            }
            RPC_RESULT_ID => {
                let req_msg_id = i64::deserialize(src)?;

                let Some(deserialize) = (params.resolve)(req_msg_id) else {
                    skip_msg!(src, end, "received response for unknown request: {req_msg_id}");
                };
                let result = unpack_object(src, &[
                    deserialize,
                    &deserialize_object::<tl::enums::RpcError>,
                ])?;

                let object = Box::new(RpcResult { req_msg_id, result });
                Ok(vec![Unpacked { msg_id, object }])
            }
            _ => {
                src.seek(Seek::Backward(4));

                match unpack_object(src, params.list) {
                    Ok(object) => Ok(vec![Unpacked { msg_id, object }]),
                    Err(deserialize::Error::IdMismatch { .. }) => {
                        skip_msg!(src, end, "received unknown object: {id:x}");
                    }
                    Err(err) => Err(err.into()),
                }
            }
        }
    }
}
