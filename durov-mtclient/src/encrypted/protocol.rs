use crate::encrypted::pool::item::Provided;
use crate::encrypted::pool::Pool;
use crate::encrypted::request::{CallData, Request};
use durov_mtproto::protocols::encrypted::object::deserialize_object;
use durov_mtproto::protocols::encrypted::{Encrypted, Packed, UnpackParams, Unpacked};
use durov_tl_types::buffer::Buffer;
use durov_tl_types::schemas::api as api_tl;
use durov_tl_types::schemas::mtproto as tl;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

pub enum ProtoAction {
    Pack(Vec<Request>),
    Unpack(Provided<Buffer>),
}

pub struct ProtoPacked {
    pub buf: Provided<Buffer>,
    pub packed: Packed,
    pub requests: Vec<Request>,
}

pub struct EncryptedWorker {
    pub bufs: Pool<Buffer>,
    pub protocol: Encrypted,
    pub proto_rx: flume::Receiver<ProtoAction>,
    pub packed_tx: flume::Sender<ProtoPacked>,
    pub unpacked_tx: flume::Sender<Result<Vec<Unpacked>, durov_mtproto::protocols::Error>>,
    pub rpc_map: Arc<Mutex<HashMap<i64, CallData>>>,
}

impl EncryptedWorker {
    pub async fn run(self) {
        loop {
            if self.step().await.is_err() {
                break;
            }
        }
    }

    async fn step(&self) -> Result<(), ()> {
        match self.proto_rx.recv_async().await {
            Ok(ProtoAction::Pack(requests)) => self.on_pack(requests)?,
            Ok(ProtoAction::Unpack(buf)) => self.on_unpack(buf)?,
            Err(flume::RecvError::Disconnected) => return Err(()),
        }

        Ok(())
    }

    fn on_pack(&self, requests: Vec<Request>) -> Result<(), ()> {
        let objects = requests.iter()
            .map(|req| match req {
                Request::Service(object) => object,
                Request::Rpc(call) => &call.body,
            })
            .collect::<Vec<_>>();

        let mut buf = self.bufs.provide();
        let packed = self.protocol.pack(&mut buf, &objects);
        self.packed_tx.send(ProtoPacked { buf, packed, requests })
            .map_err(drop)?;

        Ok(())
    }

    fn on_unpack(&self, mut buf: Provided<Buffer>) -> Result<(), ()> {
        let params = UnpackParams {
            list: &[
                &deserialize_object::<tl::enums::NewSession>,
                &deserialize_object::<tl::enums::FutureSalts>,
                &deserialize_object::<tl::enums::BadMsgNotification>,
                &deserialize_object::<tl::enums::MsgsAck>,
                &deserialize_object::<tl::enums::Pong>,
                &deserialize_object::<api_tl::enums::Updates>,
            ],
            resolve: &|msg_id| {
                self.rpc_map.lock().unwrap()
                    .get(&msg_id)
                    .map(|call| call.deserialize)
            },
        };
        let result = self.protocol.unpack(&mut buf, params);
        self.unpacked_tx.send(result)
            .map_err(drop)?;

        Ok(())
    }
}