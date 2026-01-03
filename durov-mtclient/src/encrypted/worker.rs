use crate::encrypted::ack::Ack;
use crate::encrypted::receiver::Receiver;
use crate::encrypted::salt::FutureSalts;
use crate::encrypted::sender::Sender;
use durov_mtproto::protocols::encrypted::object::{deserialize_object, DeserializeObject, InObject, Object};
use durov_mtproto::protocols::encrypted::{Encrypted, RpcResult};
use durov_mtproto::protocols::time::{get_now, parse_msg_id};
use durov_mtproto::transports::Transport;
use durov_tl_types::buffer::Buffer;
use durov_tl_types::schemas::api as api_tl;
use durov_tl_types::schemas::mtproto as tl;
use durov_tl_types::Identify;
use std::collections::HashMap;
use std::sync::Arc;
use thiserror::Error;
use tokio::io;
use tokio::net::TcpStream;
use tokio::sync::{mpsc, oneshot};

#[derive(Error, Debug)]
enum Error {
    #[error("io: {0}")]
    Io(#[from] io::Error),

    #[error("transport: {0}")]
    Transport(#[from] durov_mtproto::transports::Error),

    #[error("protocol: {0}")]
    Protocol(#[from] durov_mtproto::protocols::Error),

    #[error("stop")]
    Stop,
}

pub struct CallData {
    pub body: InObject,
    pub tx: oneshot::Sender<Option<Object>>,
    pub deserialize: DeserializeObject,
}

pub struct Worker<T> {
    sender: Sender,
    receiver: Receiver,
    transport: T,
    protocol: Encrypted,
    call_rx: mpsc::UnboundedReceiver<CallData>,
    call_map: HashMap<i64, oneshot::Sender<Option<Object>>>,
    deserialize_map: HashMap<i64, DeserializeObject>,
    updates_tx: mpsc::UnboundedSender<api_tl::enums::Updates>,
    new_session_notified: bool,
    ack: Ack,
    salts: FutureSalts,
    synced_salt: bool,
}

impl<T> Worker<T> {
    pub fn new(
        stream: TcpStream,
        transport: T,
        protocol: Encrypted,
        call_rx: mpsc::UnboundedReceiver<CallData>,
        updates_tx: mpsc::UnboundedSender<api_tl::enums::Updates>,
    ) -> Self {
        let (reader, writer) = stream.into_split();
        Self {
            sender: Sender::new(writer),
            receiver: Receiver::new(reader),
            transport,
            protocol,
            call_rx,
            call_map: HashMap::new(),
            deserialize_map: HashMap::new(),
            updates_tx,
            new_session_notified: false,
            ack: Ack::new(),
            salts: FutureSalts::new(),
            synced_salt: false,
        }
    }
}

impl<T: Transport> Worker<T> {
    pub async fn run(mut self) {
        loop {
            if let Err(err) = self.step().await {
                match err {
                    Error::Stop => log::info!("worker stopped"),
                    _ => log::error!("worker: {err}"),
                }
                break;
            }
        }
    }

    async fn step(&mut self) -> Result<(), Error> {
        tokio::select! {
            n = self.receiver.select() => {
                self.on_recv(n?)
            }
            n = self.sender.select(), if self.sender.condition() => {
                self.on_send(n?)
            }
            call = self.call_rx.recv(), if self.protocol.is_ready() => {
                self.on_call(call.ok_or(Error::Stop)?)
            }
            _ = self.ack.select(), if self.ack.condition() => {
                self.on_ack_timeout()
            }
            _ = self.salts.select() => {
                self.on_future_salt()
            }
        }
    }

    fn on_recv(&mut self, n: usize) -> Result<(), Error> {
        self.receiver.pos += n;

        if self.receiver.pos == self.receiver.buf.len() {
            self.process_recv_buf()?;
        }

        Ok(())
    }

    fn on_send(&mut self, n: usize) -> Result<(), Error> {
        self.sender.pos += n;

        if self.sender.pos == self.sender.bufs[0].len() {
            self.sender.bufs.pop_front();
            self.sender.pos = 0;
        }

        Ok(())
    }

    fn on_call(&mut self, call: CallData) -> Result<(), Error> {
        let msg_ids = if self.ack.condition() {
            let ack = self.new_ack_object();
            self.enqueue_objects(&[call.body, ack])
        } else {
            self.enqueue_objects(&[call.body])
        };
        self.call_map.insert(msg_ids[0], call.tx);
        self.deserialize_map.insert(msg_ids[0], call.deserialize);

        Ok(())
    }

    fn on_ack_timeout(&mut self) -> Result<(), Error> {
        let object = self.new_ack_object();
        self.enqueue_objects(&[object]);

        Ok(())
    }

    fn on_future_salt(&mut self) -> Result<(), Error> {
        if self.salts.can_get() {
            let salt = self.salts.pop();
            self.protocol.set_salt(salt);
        } else {
            let object = tl::functions::GetFutureSalts { num: 4 };
            let object = InObject::new(Arc::new(object));
            self.enqueue_objects(&[object]);
            self.salts.asked = get_now();
        }

        Ok(())
    }

    fn new_ack_object(&mut self) -> InObject {
        let object = tl::enums::MsgsAck::MsgsAck(
            tl::types::MsgsAck {
                msg_ids: self.ack.next_batch(),
            }
        );
        InObject {
            id: tl::types::MsgsAck::ID,
            body: Arc::new(object),
        }
    }

    fn process_recv_buf(&mut self) -> Result<(), Error> {
        match self.transport.unpack(&mut self.receiver.buf) {
            Ok(()) => {
                let objects = self.protocol.unpack(
                    &mut self.receiver.buf,
                    &[
                        deserialize_object::<tl::enums::NewSession>,
                        deserialize_object::<tl::enums::FutureSalts>,
                        deserialize_object::<tl::enums::BadMsgNotification>,
                        deserialize_object::<tl::enums::MsgsAck>,
                        deserialize_object::<api_tl::enums::Updates>,
                    ],
                    &self.deserialize_map,
                )?;

                for obj in objects {
                    self.process_object(obj.msg_id, obj.body);
                }

                self.receiver.buf.clear();
                self.receiver.pos = 0;
            }
            Err(durov_mtproto::transports::Error::MissingBytes(missing)) => {
                self.receiver.buf.resize_back(missing);
            }
            Err(err) => return Err(err.into()),
        }

        Ok(())
    }

    fn process_object(&mut self, msg_id: i64, body: Object) {
        match body.downcast::<RpcResult>() {
            Ok(rpc) => {
                let tx = self.call_map.remove(&rpc.req_msg_id)
                    .expect("this check should be done in protocol unpack flow");
                tx.send(Some(rpc.result)).ok();
                self.deserialize_map.remove(&rpc.req_msg_id);
                self.ack.add(msg_id);
            }
            Err(body) => self.process_new_session(msg_id, body),
        }
    }

    fn process_new_session(&mut self, msg_id: i64, body: Object) {
        match body.downcast::<tl::enums::NewSession>() {
            Ok(new) => {
                let tl::enums::NewSession::NewSessionCreated(new) = *new;

                self.protocol.set_salt(new.server_salt);

                if self.new_session_notified {
                    // client should pull missed updates
                    // maybe in the future I will implement this
                    log::warn!("received new session notification again");
                } else {
                    self.new_session_notified = true;
                }

                self.ack.add(msg_id);
            }
            Err(body) => self.process_future_salts(msg_id, body),
        }
    }

    fn process_future_salts(&mut self, msg_id: i64, body: Object) {
        match body.downcast::<tl::enums::FutureSalts>() {
            Ok(future) => {
                let tl::enums::FutureSalts::FutureSalts(future) = *future;

                for salt in future.salts.0 {
                    let now = get_now();
                    let server_now = future.now as f64;
                    let diff = server_now - now;

                    let server_since = salt.valid_since as f64;
                    let since = server_since - diff;

                    self.salts.add(salt.salt, since);
                }
            }
            Err(body) => self.process_bad_msg_notification(msg_id, body),
        }
    }

    fn process_bad_msg_notification(&mut self, msg_id: i64, body: Object) {
        match body.downcast::<tl::enums::BadMsgNotification>() {
            Ok(bad) => {
                match *bad {
                    tl::enums::BadMsgNotification::BadMsgNotification(bad) => {
                        if matches!(bad.error_code, 16 | 17) {
                            let server_time = parse_msg_id(msg_id);
                            self.protocol.set_server_time(server_time);
                        }
                        self.apply_bad_msg_notification(bad.bad_msg_id, bad.error_code);
                    }
                    tl::enums::BadMsgNotification::BadServerSalt(bad) => {
                        self.protocol.set_salt(bad.new_server_salt);
                        self.apply_bad_msg_notification(bad.bad_msg_id, bad.error_code);
                    }
                }
            }
            Err(body) => self.process_messages_ack(body),
        }
    }

    fn apply_bad_msg_notification(&mut self, msg_id: i64, code: i32) {
        if let Some(tx) = self.call_map.remove(&msg_id) {
            tx.send(None).ok();
            self.deserialize_map.remove(&msg_id);
            log::warn!("received bad msg notification for request: {code}");
        } else if code == 48 && !self.synced_salt {
            self.synced_salt = true;
        } else {
            log::warn!("received bad msg notification for service message or unknown request: {code}");
        }
    }

    fn process_messages_ack(&mut self, body: Object) {
        match body.downcast::<tl::enums::MsgsAck>() {
            Ok(_) => (),
            Err(body) => self.process_updates(body),
        }
    }

    fn process_updates(&mut self, body: Object) {
        match body.downcast::<api_tl::enums::Updates>() {
            Ok(updates) => { self.updates_tx.send(*updates).ok(); }
            Err(_) => unreachable!("this check should be done in protocol unpack flow"),
        }
    }

    fn enqueue_objects(&mut self, objects: &[InObject]) -> Vec<i64> {
        let mut message_ids = Vec::new();
        for chunk in objects.chunks(1024) {
            let mut buf = Buffer::new();
            let msg_ids = self.protocol.pack(&mut buf, chunk);
            self.transport.pack(&mut buf);
            self.sender.bufs.push_back(buf);
            message_ids.extend(msg_ids);
        }
        message_ids
    }
}
