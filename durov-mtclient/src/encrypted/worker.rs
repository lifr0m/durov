use crate::encrypted::ack::Ack;
use crate::encrypted::complications::redirect_updates;
use crate::encrypted::helpers::Chunks;
use crate::encrypted::pool::Pool;
use crate::encrypted::protocol::{EncryptedWorker, ProtoAction, ProtoPacked};
use crate::encrypted::receiver::Receiver;
use crate::encrypted::request::{CallData, Request};
use crate::encrypted::salt::FutureSalts;
use crate::encrypted::sender::Sender;
use crate::encrypted::timed::Timed;
use durov_mtproto::protocols::encrypted::object::{PackObject, UnpackObject};
use durov_mtproto::protocols::encrypted::{Encrypted, RpcResult, Unpacked};
use durov_mtproto::protocols::time::{get_now, parse_msg_id};
use durov_mtproto::transports::Transport;
use durov_tl_types::buffer::Buffer;
use durov_tl_types::schemas::api as api_tl;
use durov_tl_types::schemas::mtproto as tl;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use std::{iter, mem, thread};
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

pub struct Worker<T> {
    bufs: Pool<Buffer>,

    sender: Sender,
    receiver: Receiver,
    transport: T,
    protocol: Encrypted,

    req_tick: time::Interval,
    req_tx: flume::Sender<Request>,
    req_rx: flume::Receiver<Request>,
    proto_tx: flume::Sender<ProtoAction>,
    packed_rx: flume::Receiver<ProtoPacked>,
    unpacked_rx: flume::Receiver<Result<Vec<Unpacked>, durov_mtproto::protocols::Error>>,

    container_map: HashMap<i64, Timed<Vec<i64>>>,
    service_map: HashMap<i64, Timed<PackObject>>,
    rpc_map: Arc<Mutex<HashMap<i64, CallData>>>,
    msg_expiration: time::Interval,

    updates_tx: Option<flume::Sender<api_tl::enums::Updates>>,

    ack: Ack,
    salts: FutureSalts,
    ping: time::Interval,
}

impl<T: Transport> Worker<T> {
    pub fn new(
        stream: TcpStream,
        transport: T,
        protocol: Encrypted,
        req_tx: flume::Sender<Request>,
        req_rx: flume::Receiver<Request>,
        updates_tx: Option<flume::Sender<api_tl::enums::Updates>>,
    ) -> Self {
        let bufs = Pool::new(Buffer::clear, Duration::from_mins(5));
        let (reader, writer) = stream.into_split();
        let (proto_tx, proto_rx) = flume::unbounded();
        let (packed_tx, packed_rx) = flume::unbounded();
        let (unpacked_tx, unpacked_rx) = flume::unbounded();
        let rpc_map = Arc::new(Mutex::new(HashMap::new()));

        let cpu_count = thread::available_parallelism().unwrap().get();
        for _ in 0..cpu_count {
            let worker = EncryptedWorker {
                bufs: bufs.clone(),
                protocol: protocol.clone(),
                rpc_map: Arc::clone(&rpc_map),
                proto_rx: proto_rx.clone(),
                packed_tx: packed_tx.clone(),
                unpacked_tx: unpacked_tx.clone(),
            };
            tokio::spawn(worker.run());
        }

        Self {
            bufs: bufs.clone(),
            sender: Sender::new(writer),
            receiver: Receiver::new(reader, bufs.provide()),
            transport,
            protocol,
            req_tick: {
                let mut interval = time::interval(Duration::from_millis(16));
                interval.set_missed_tick_behavior(MissedTickBehavior::Delay);
                interval
            },
            req_tx,
            req_rx,
            proto_tx,
            packed_rx,
            unpacked_rx,
            container_map: HashMap::new(),
            service_map: HashMap::new(),
            rpc_map,
            msg_expiration: {
                let mut interval = time::interval(Duration::from_secs(1));
                interval.set_missed_tick_behavior(MissedTickBehavior::Delay);
                interval
            },
            updates_tx,
            ack: Ack::new(),
            salts: FutureSalts::new(),
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
                Err(Error::Stop) => {}
                Err(err) => tracing::error!("worker: {err}"),
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
            _ = self.req_tick.tick() => {
                self.on_req_tick()?;
            }
            packed = self.packed_rx.recv_async() => {
                self.on_packed(packed.expect("protocol worker should not stop"));
            }
            list = self.unpacked_rx.recv_async() => {
                self.on_unpacked(list.expect("protocol worker should not stop")?);
            }
            _ = self.msg_expiration.tick() => {
                self.on_msg_expiration();
            }
            _ = self.ack.wait(), if self.ack.condition() => {
                self.on_ack_timeout();
            }
            _ = self.salts.wait() => {
                self.on_future_salt();
            }
            _ = self.ping.tick() => {
                self.on_ping();
            }
        }

        Ok(())
    }

    fn on_recv(&mut self, n: usize) -> Result<(), Error> {
        self.receiver.limit -= n;

        if self.receiver.limit == 0 {
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

    fn on_req_tick(&mut self) -> Result<(), Error> {
        let mut requests = iter::repeat_with(|| self.req_rx.try_recv())
            .map_while(|req| match req {
                Ok(req) => Some(Ok(req)),
                Err(flume::TryRecvError::Empty) => None,
                Err(flume::TryRecvError::Disconnected) => Some(Err(Error::Stop)),
            })
            .collect::<Result<Vec<_>, _>>()?;

        if !requests.is_empty() {
            requests.extend(self.new_ack_requests());
            for chunk in requests.into_iter().chunks(1024) {
                self.proto_tx.send(ProtoAction::Pack(chunk))
                    .expect("protocol worker should not stop");
            }
        }

        Ok(())
    }

    fn on_packed(&mut self, mut packed: ProtoPacked) {
        self.transport.pack(&mut packed.buf);
        self.sender.bufs.push_back(packed.buf);

        if let Some(msg_id) = packed.packed.container_msg_id {
            self.container_map.insert(msg_id, Timed::new(packed.packed.msg_ids.clone()));
        }

        let mut rpc_map = self.rpc_map.lock().unwrap();

        for (msg_id, req) in iter::zip(packed.packed.msg_ids, packed.requests) {
            match req {
                Request::Service(object) => {
                    self.service_map.insert(msg_id, Timed::new(object));
                }
                Request::Rpc(call) => {
                    rpc_map.insert(msg_id, call);
                }
            }
        }
    }

    fn on_unpacked(&mut self, list: Vec<Unpacked>) {
        for unpacked in list {
            self.process_object(unpacked.msg_id, unpacked.object);
        }
    }

    fn on_msg_expiration(&mut self) {
        let timeout = Duration::from_secs(5);
        self.container_map.retain(|_, value| !value.expired(timeout));
        self.service_map.retain(|_, value| !value.expired(timeout));
    }

    fn on_ack_timeout(&mut self) {
        for req in self.new_ack_requests() {
            self.req_tx.send(req)
                .expect("we are in running worker");
        }
    }

    fn on_future_salt(&mut self) {
        if self.salts.can_get() {
            let salt = self.salts.pop();
            self.protocol.set_salt(salt);
        } else {
            let object = tl::functions::GetFutureSalts { num: 4 };
            self.req_tx.send(Request::Service(Box::new(object)))
                .expect("we are in running worker");
            self.salts.asked = get_now();
        }
    }

    fn on_ping(&mut self) {
        let object = tl::functions::PingDelayDisconnect {
            ping_id: rand::random(),
            disconnect_delay: 75,
        };
        self.req_tx.send(Request::Service(Box::new(object)))
            .expect("we are in running worker");
    }

    fn new_ack_requests(&mut self) -> Vec<Request> {
        iter::repeat_with(|| self.ack.next_batch())
            .take_while(|msg_ids| !msg_ids.is_empty())
            .map(|msg_ids| tl::enums::MsgsAck::MsgsAck(tl::types::MsgsAck { msg_ids }))
            .map(|object| Request::Service(Box::new(object)))
            .collect()
    }

    fn process_recv_buf(&mut self) -> Result<(), Error> {
        match self.transport.unpack(&mut self.receiver.buf) {
            Ok(()) => {
                let buf = mem::replace(&mut self.receiver.buf, self.bufs.provide());
                self.proto_tx.send(ProtoAction::Unpack(buf))
                    .expect("protocol worker should not stop");
            }
            Err(durov_mtproto::transports::Error::MissingBytes(missing)) => {
                self.receiver.limit += missing;
            }
            Err(err) => return Err(err.into()),
        }

        Ok(())
    }

    fn process_object(&mut self, msg_id: i64, object: UnpackObject) {
        match object.downcast::<RpcResult>() {
            Ok(mut rpc) => {
                let call = self.rpc_map.lock().unwrap().remove(&rpc.req_msg_id)
                    .expect("this check should be done in protocol unpack flow");
                if let Some(updates_tx) = &self.updates_tx {
                    redirect_updates(updates_tx, call.body.as_ref(), &mut rpc.result);
                }
                call.callback.send(rpc.result).ok();
                self.ack.add(msg_id);
            }
            Err(object) => self.process_new_session(msg_id, object),
        }
    }

    fn process_new_session(&mut self, msg_id: i64, object: UnpackObject) {
        match object.downcast::<tl::enums::NewSession>() {
            Ok(new) => {
                let tl::enums::NewSession::NewSessionCreated(new) = *new;

                self.protocol.set_salt(new.server_salt);

                if let Some(updates_tx) = &self.updates_tx {
                    updates_tx.send(api_tl::types::UpdatesTooLong {}.into()).ok();
                }

                self.ack.add(msg_id);
            }
            Err(object) => self.process_future_salts(msg_id, object),
        }
    }

    fn process_future_salts(&mut self, msg_id: i64, object: UnpackObject) {
        match object.downcast::<tl::enums::FutureSalts>() {
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
            Err(object) => self.process_bad_msg_notification(msg_id, object),
        }
    }

    fn process_bad_msg_notification(&mut self, msg_id: i64, object: UnpackObject) {
        match object.downcast::<tl::enums::BadMsgNotification>() {
            Ok(bad) => {
                match *bad {
                    tl::enums::BadMsgNotification::BadMsgNotification(bad) => {
                        if matches!(bad.error_code, 16 | 17) {
                            let server_time = parse_msg_id(msg_id);
                            self.protocol.set_server_time(server_time);
                        }
                        self.apply_bad_msg_notification(bad.bad_msg_id, bad.error_code, true);
                    }
                    tl::enums::BadMsgNotification::BadServerSalt(bad) => {
                        self.protocol.set_salt(bad.new_server_salt);
                        self.apply_bad_msg_notification(bad.bad_msg_id, bad.error_code, true);
                    }
                }
            }
            Err(object) => self.process_messages_ack(object),
        }
    }

    fn apply_bad_msg_notification(&mut self, msg_id: i64, code: i32, log: bool) {
        if let Some(message_ids) = self.container_map.remove(&msg_id) {
            for msg_id in message_ids.value {
                self.apply_bad_msg_notification(msg_id, code, false);
            }
            tracing::warn!(code, "received bad msg notification for container");
        } else if let Some(object) = self.service_map.remove(&msg_id) {
            self.req_tx.send(Request::Service(object.value))
                .expect("we are in running worker");
            if log {
                tracing::warn!(code, "received bad msg notification for service message");
            }
        } else if let Some(call) = self.rpc_map.lock().unwrap().remove(&msg_id) {
            self.req_tx.send(Request::Rpc(call))
                .expect("we are in running worker");
            if log {
                tracing::warn!(code, "received bad msg notification for rpc call");
            }
        } else {
            tracing::warn!(code, "received bad msg notification for unknown request");
        }
    }

    fn process_messages_ack(&mut self, object: UnpackObject) {
        match object.downcast::<tl::enums::MsgsAck>() {
            Ok(_) => {}
            Err(object) => self.process_pong(object),
        }
    }

    fn process_pong(&mut self, object: UnpackObject) {
        match object.downcast::<tl::enums::Pong>() {
            Ok(_) => {}
            Err(object) => self.process_updates(object),
        }
    }

    fn process_updates(&mut self, object: UnpackObject) {
        match object.downcast::<api_tl::enums::Updates>() {
            Ok(updates) => match &self.updates_tx {
                Some(updates_tx) => {
                    updates_tx.send(*updates).ok();
                }
                None => tracing::warn!("server sent updates while in no-updates mode"),
            }
            Err(_) => unreachable!("this check should be done in protocol unpack flow"),
        }
    }
}
