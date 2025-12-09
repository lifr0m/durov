mod worker;
mod sender;
mod receiver;
mod ack;
mod salt;

use crate::{tcp, Error, MtConfig};
use durov_mtproto::protocols::encrypted::object::{deserialize_object, InObject, Object};
use durov_mtproto::protocols::encrypted::Encrypted;
use durov_mtproto::transports::Transport;
use durov_tl_types::deserialize::Deserialize;
use durov_tl_types::schemas::mtproto as tl;
use durov_tl_types::serialize::Serialize;
use durov_tl_types::{Call, Identify};
use tokio::net::TcpStream;
use tokio::sync::{mpsc, oneshot};
use worker::{CallData, Worker};

pub struct EncryptedClient {
    call_tx: mpsc::UnboundedSender<CallData>,
}

impl EncryptedClient {
    pub fn new<T>(stream: TcpStream, transport: T, protocol: Encrypted) -> Self
    where
        T: Transport + Send + 'static,
    {
        let (call_tx, call_rx) = mpsc::unbounded_channel();
        tokio::spawn(Worker::new(stream, transport, protocol, call_rx).run());
        Self { call_tx }
    }

    pub async fn connect<T>(config: MtConfig, auth_key: [u8; 256]) -> Result<Self, Error>
    where
        T: Transport + Send + 'static,
    {
        let stream = tcp::connect(config.dc.host, config.dc.port).await?;
        let transport = T::default();
        let protocol = Encrypted::new(auth_key, config.use_gzip);
        Ok(Self::new(stream, transport, protocol))
    }

    pub async fn call<F>(&self, func: F) -> Result<F::Result, Error>
    where
        F: Identify + Call + Serialize + Send + 'static,
        F::Result: Deserialize + Send,
    {
        let (tx, rx) = oneshot::channel();

        let call = CallData {
            body: InObject::new(func),
            tx,
            deserialize: deserialize_object::<F::Result>,
        };
        if self.call_tx.send(call).is_err() {
            return Err(Error::Connection);
        }

        let Ok(object) = rx.await else {
            return Err(Error::Connection);
        };
        let Some(object) = object else {
            return Err(Error::Resend);
        };
        self.process_rpc_response::<F>(object)
    }

    fn process_rpc_response<F: Call>(&self, object: Object) -> Result<F::Result, Error>
    where
        F::Result: 'static,
    {
        match object.downcast::<F::Result>() {
            Ok(result) => Ok(*result),
            Err(object) => self.process_rpc_error::<F>(object),
        }
    }

    fn process_rpc_error<F: Call>(&self, object: Object) -> Result<F::Result, Error> {
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
