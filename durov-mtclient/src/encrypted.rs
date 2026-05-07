mod worker;
mod sender;
mod receiver;
mod ack;
mod salt;
mod complications;

use crate::config::MtConfig;
use crate::{tcp, Error};
use durov_mtproto::protocols::encrypted::object::{deserialize_object, UnpackObject};
use durov_mtproto::protocols::encrypted::Encrypted;
use durov_mtproto::transports::Transport;
use durov_tl_types::deserialize::Deserialize;
use durov_tl_types::schemas::api as api_tl;
use durov_tl_types::schemas::mtproto as tl;
use durov_tl_types::serialize::Serialize;
use durov_tl_types::{Call, Identify};
use std::marker::PhantomData;
use tokio::net::TcpStream;
use worker::{CallData, Worker};

pub struct EncryptedClient<T> {
    call_tx: flume::Sender<CallData>,
    updates_rx: flume::Receiver<api_tl::enums::Updates>,
    transport: PhantomData<T>,
}

impl<T: Transport> EncryptedClient<T>
where
    T: Send + 'static,
{
    pub fn new(stream: TcpStream, transport: T, protocol: Encrypted, updates: bool) -> Self {
        let (call_tx, call_rx) = flume::unbounded();
        let (updates_tx, updates_rx) = flume::unbounded();
        tokio::spawn(Worker::new(stream, transport, protocol, call_rx, updates.then_some(updates_tx)).run());
        Self { call_tx, updates_rx, transport: PhantomData }
    }

    pub async fn connect(config: MtConfig, auth_key: [u8; 256]) -> Result<Self, Error> {
        let stream = tcp::connect(&config.dc, config.proxy.as_ref()).await?;
        let transport = T::default();
        let protocol = Encrypted::new(auth_key, config.use_gzip);
        Ok(Self::new(stream, transport, protocol, config.updates))
    }

    pub async fn call<F>(&self, func: F) -> Result<F::Result, Error>
    where
        F: Identify + Call + Serialize + Send + 'static,
        F::Result: Deserialize + Send,
    {
        let (tx, rx) = flume::unbounded();

        let call = CallData {
            body: Box::new(func),
            callback: tx,
            deserialize: &deserialize_object::<F::Result>,
        };
        if self.call_tx.send(call).is_err() {
            return Err(Error::Connection);
        }

        let object = rx.recv_async().await
            .map_err(|_| Error::Connection)?;
        self.process_rpc_response(object)
    }

    pub async fn next(&self) -> Result<api_tl::enums::Updates, Error> {
        self.updates_rx.recv_async().await
            .map_err(|_| Error::Connection)
    }

    fn process_rpc_response<R>(&self, object: UnpackObject) -> Result<R, Error>
    where
        R: 'static,
    {
        match object.downcast::<R>() {
            Ok(result) => Ok(*result),
            Err(object) => self.process_rpc_error(object),
        }
    }

    fn process_rpc_error<R>(&self, object: UnpackObject) -> Result<R, Error> {
        match object.downcast::<tl::enums::RpcError>() {
            Ok(rpc) => {
                let tl::enums::RpcError::RpcError(rpc) = *rpc;

                Err(Error::RpcError {
                    code: rpc.error_code,
                    message: rpc.error_message,
                })
            }
            Err(_) => unreachable!("this check should be done in protocol unpack flow"),
        }
    }
}
