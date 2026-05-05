use crate::encrypted::ack::Ack;
use crate::encrypted::complications::redirect_updates;
use crate::encrypted::receiver::Receiver;
use crate::encrypted::salt::FutureSalts;
use crate::encrypted::sender::Sender;
use durov_mtproto::protocols::encrypted::object::{deserialize_box, DeserializeBox, PackObject, UnpackObject};
use durov_mtproto::protocols::encrypted::{Encrypted, RpcResult, UnpackParams};
use durov_mtproto::protocols::time::{get_now, parse_msg_id};
use durov_mtproto::transports::Transport;
use durov_tl_types::buffer::Buffer;
use durov_tl_types::schemas::api as api_tl;
use durov_tl_types::schemas::mtproto as tl;
use std::collections::HashMap;
use std::time::Duration;
use thiserror::Error;
use tokio::net::TcpStream;
use tokio::time::MissedTickBehavior;
use tokio::{io, time};

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
    pub body: PackObject,
    pub callback: flume::Sender<UnpackObject>,
    pub deserialize: DeserializeBox,
}

pub struct Worker<T> {
    sender: Sender,
    receiver: Receiver,
    transport: T,
    protocol: Encrypted,
    call_rx: flume::Receiver<CallData>,
    call_map: HashMap<i64, CallData>,
    updates_tx: Option<flume::Sender<api_tl::enums::Updates>>,
    ack: Ack,
    salts: FutureSalts,
    synced_salt: bool,
    ping: time::Interval,
}

impl<T: Transport> Worker<T> {
    pub fn new(
        stream: TcpStream,
        transport: T,
        protocol: Encrypted,
        call_rx: flume::Receiver<CallData>,
        updates_tx: Option<flume::Sender<api_tl::enums::Updates>>,
    ) -> Self {
        let (reader, writer) = stream.into_split();
        Self {
            sender: Sender::new(writer),
            receiver: Receiver::new(reader),
            transport,
            protocol,
            call_rx,
            call_map: HashMap::new(),
            updates_tx,
            ack: Ack::new(),
            salts: FutureSalts::new(),
            synced_salt: false,
            ping: {
                let mut interval = time::interval(Duration::from_secs(60));
                interval.set_missed_tick_behavior(MissedTickBehavior::Delay);
                interval
            },
        }
    }

    pub async fn run(mut self) {
        loop {
            match self.step().await {
                Ok(()) => continue,
                Err(Error::Stop) => log::info!("worker stopped"),
                Err(err) => log::error!("worker: {err}"),
            }
            break;
        }
    }

    async fn step(&mut self) -> Result<(), Error> {
        tokio::select! {
            n = self.receiver.recv() => {
                self.on_recv(n?)?;
            }
            n = self.sender.send(), if self.sender.condition() => {
                self.on_send(n?);
            }
            call = self.call_rx.recv_async(), if self.protocol.is_ready() => {
                self.on_call(call.map_err(|_| Error::Stop)?);
            }
            _ = self.ack.wait(), if self.protocol.is_ready() && self.ack.condition() => {
                self.on_ack_timeout();
            }
            _ = self.salts.wait() => {
                self.on_future_salt();
            }
            _ = self.ping.tick(), if self.protocol.is_ready() => {
                self.on_ping();
            }
        }

        Ok(())
    }

    fn on_recv(&mut self, n: usize) -> Result<(), Error> {
        self.receiver.pos += n;

        if self.receiver.pos == self.receiver.buf.len() {
            self.process_recv_buf()?;
        }

        Ok(())
    }

    fn on_send(&mut self, n: usize) {
        self.sender.pos += n;

        if self.sender.pos == self.sender.bufs[0].len() {
            self.sender.bufs.pop_front();
            self.sender.pos = 0;
        }
    }

    fn on_call(&mut self, call: CallData) {
        let msg_ids = if self.ack.condition() {
            let ack = self.new_ack_object();
            self.enqueue_objects(&[&call.body, &ack])
        } else {
            self.enqueue_objects(&[&call.body])
        };
        self.call_map.insert(msg_ids[0], call);
    }

    fn on_ack_timeout(&mut self) {
        let object = self.new_ack_object();
        self.enqueue_objects(&[&object]);
    }

    fn on_future_salt(&mut self) {
        if self.salts.can_get() {
            let salt = self.salts.pop();
            self.protocol.set_salt(salt);
        } else {
            let object = tl::functions::GetFutureSalts { num: 4 };
            let object = Box::new(object) as PackObject;
            self.enqueue_objects(&[&object]);
            self.salts.asked = get_now();
        }
    }

    fn on_ping(&mut self) {
        let object = tl::functions::PingDelayDisconnect {
            ping_id: rand::random(),
            disconnect_delay: 75,
        };
        let object = Box::new(object) as PackObject;
        self.enqueue_objects(&[&object]);
    }

    fn new_ack_object(&mut self) -> PackObject {
        let object = tl::enums::MsgsAck::MsgsAck(
            tl::types::MsgsAck {
                msg_ids: self.ack.next_batch(),
            }
        );
        Box::new(object)
    }

    fn process_recv_buf(&mut self) -> Result<(), Error> {
        match self.transport.unpack(&mut self.receiver.buf) {
            Ok(()) => {
                let params = UnpackParams {
                    list: &[
                        deserialize_box::<tl::enums::NewSession>,
                        deserialize_box::<tl::enums::FutureSalts>,
                        deserialize_box::<tl::enums::BadMsgNotification>,
                        deserialize_box::<tl::enums::MsgsAck>,
                        deserialize_box::<tl::enums::Pong>,
                        deserialize_box::<api_tl::enums::Updates>,
                    ],
                    resolve: |msg_id| {
                        self.call_map.get(&msg_id)
                            .map(|call| call.deserialize)
                    },
                };
                let list = self.protocol.unpack(&mut self.receiver.buf, params)?;

                for unpacked in list {
                    self.process_object(unpacked.msg_id, unpacked.object);
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

    fn process_object(&mut self, msg_id: i64, body: UnpackObject) {
        match body.downcast::<RpcResult>() {
            Ok(mut rpc) => {
                let call = self.call_map.remove(&rpc.req_msg_id)
                    .expect("this check should be done in protocol unpack flow");
                if let Some(updates_tx) = &self.updates_tx {
                    redirect_updates(updates_tx, call.body.as_ref(), &mut rpc.result);
                }
                call.callback.send(rpc.result).ok();
                self.ack.add(msg_id);
            }
            Err(body) => self.process_new_session(msg_id, body),
        }
    }

    fn process_new_session(&mut self, msg_id: i64, body: UnpackObject) {
        match body.downcast::<tl::enums::NewSession>() {
            Ok(new) => {
                let tl::enums::NewSession::NewSessionCreated(new) = *new;

                self.protocol.set_salt(new.server_salt);

                if let Some(updates_tx) = &self.updates_tx {
                    updates_tx.send(api_tl::types::UpdatesTooLong {}.into()).ok();
                }

                self.ack.add(msg_id);
            }
            Err(body) => self.process_future_salts(msg_id, body),
        }
    }

    fn process_future_salts(&mut self, msg_id: i64, body: UnpackObject) {
        match body.downcast::<tl::enums::FutureSalts>() {
            Ok(future) => {
                let tl::enums::FutureSalts::FutureSalts(mut future) = *future;

                future.salts.0.sort_by_key(|salt| salt.valid_since);

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

    fn process_bad_msg_notification(&mut self, msg_id: i64, body: UnpackObject) {
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
        if let Some(call) = self.call_map.remove(&msg_id) {
            self.on_call(call);
            log::warn!("received bad msg notification for request: {code}");
        } else if code == 48 && !self.synced_salt {
            self.synced_salt = true;
        } else {
            log::warn!("received bad msg notification for service message or unknown request: {code}");
        }
    }

    fn process_messages_ack(&mut self, body: UnpackObject) {
        match body.downcast::<tl::enums::MsgsAck>() {
            Ok(_) => {}
            Err(body) => self.process_pong(body),
        }
    }

    fn process_pong(&mut self, body: UnpackObject) {
        match body.downcast::<tl::enums::Pong>() {
            Ok(_) => {}
            Err(body) => self.process_updates(body),
        }
    }

    fn process_updates(&mut self, body: UnpackObject) {
        match body.downcast::<api_tl::enums::Updates>() {
            Ok(updates) => match &self.updates_tx {
                Some(updates_tx) => { updates_tx.send(*updates).ok(); }
                None => log::warn!("server sent updates while in no-updates mode"),
            }
            Err(_) => unreachable!("this check should be done in protocol unpack flow"),
        }
    }

    fn enqueue_objects(&mut self, objects: &[&PackObject]) -> Vec<i64> {
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
