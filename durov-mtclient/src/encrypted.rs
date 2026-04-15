mod worker;
mod sender;
mod receiver;
mod ack;
mod salt;
mod complications;

use crate::config::MtConfig;
use crate::{tcp, Error};
use durov_mtproto::protocols::encrypted::object::{deserialize_object, InObject, Object};
use durov_mtproto::protocols::encrypted::Encrypted;
use durov_mtproto::transports::Transport;
use durov_tl_types::deserialize::Deserialize;
use durov_tl_types::schemas::api as api_tl;
use durov_tl_types::schemas::mtproto as tl;
use durov_tl_types::serialize::Serialize;
use durov_tl_types::{Call, Identify};
use std::marker::PhantomData;
use tokio::net::TcpStream;
use tokio::sync::{mpsc, oneshot, Mutex};
use worker::{CallData, Worker};

pub struct EncryptedClient<T> {
    call_tx: mpsc::UnboundedSender<CallData>,
    updates_rx: Mutex<mpsc::UnboundedReceiver<api_tl::enums::Updates>>,
    transport: PhantomData<T>,
}

impl<T: Transport> EncryptedClient<T>
where
    T: Send + 'static,
{
    pub fn new(stream: TcpStream, transport: T, protocol: Encrypted, updates: bool) -> Self {
        let (call_tx, call_rx) = mpsc::unbounded_channel();
        let (updates_tx, updates_rx) = mpsc::unbounded_channel();
        tokio::spawn(Worker::new(stream, transport, protocol, call_rx, updates.then_some(updates_tx)).run());
        Self {
            call_tx,
            updates_rx: Mutex::new(updates_rx),
            transport: PhantomData,
        }
    }

    pub async fn connect(config: MtConfig, auth_key: [u8; 256]) -> Result<Self, Error> {
        let stream = tcp::connect(&config.dc.host, config.dc.port).await?;
        let transport = T::default();
        let protocol = Encrypted::new(auth_key, config.use_gzip);
        Ok(Self::new(stream, transport, protocol, config.updates))
    }

    pub async fn call<F>(&self, func: F) -> Result<F::Result, Error>
    where
        F: Identify + Call + Serialize + Send + 'static,
        F::Result: Deserialize + Send,
    {
        let (tx, rx) = oneshot::channel();

        let call = CallData {
            body: InObject::new(func),
            callback: tx,
            deserialize: deserialize_object::<F::Result>,
        };
        if self.call_tx.send(call).is_err() {
            return Err(Error::Connection);
        }

        let object = rx.await
            .map_err(|_| Error::Connection)?;
        self.process_rpc_response(object)
    }

    pub async fn next(&self) -> Result<api_tl::enums::Updates, Error> {
        self.updates_rx.try_lock()
            .expect("you can listen for updates only from one task")
            .recv()
            .await
            .ok_or(Error::Connection)
    }

    fn process_rpc_response<R>(&self, object: Object) -> Result<R, Error>
    where
        R: 'static,
    {
        match object.downcast::<R>() {
            Ok(result) => Ok(*result),
            Err(object) => self.process_rpc_error(object),
        }
    }

    fn process_rpc_error<R>(&self, object: Object) -> Result<R, Error> {
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
