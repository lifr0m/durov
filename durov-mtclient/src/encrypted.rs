mod worker;
mod sender;
mod receiver;
mod ack;
mod salt;

use crate::{tcp, Config, Error};
use durov_mtproto::protocols::encrypted::object::InObject;
use durov_mtproto::protocols::encrypted::Encrypted;
use durov_mtproto::transports::Transport;
use durov_tl_types::schemas::mtproto as tl;
use durov_tl_types::serialize::Serialize;
use durov_tl_types::{Call, Identify, Object};
use std::any::Any;
use tokio::io;
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

    pub async fn connect<T>(config: Config, auth_key: [u8; 256]) -> io::Result<Self>
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
    {
        let (tx, rx) = oneshot::channel();

        let call = CallData {
            body: InObject::new(func),
            tx,
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

    fn process_rpc_response<F>(&self, object: Object) -> Result<F::Result, Error>
    where
        F: Identify + Call,
        F::Result: 'static,
    {
        match object.body.downcast::<F::Result>() {
            Ok(result) => Ok(*result),
            Err(body) => self.process_rpc_error::<F>(object.id, body),
        }
    }

    fn process_rpc_error<F>(&self, id: i32, body: Box<dyn Any>) -> Result<F::Result, Error>
    where
        F: Identify + Call,
    {
        match body.downcast::<tl::enums::RpcError>() {
            Ok(rpc_error) => {
                let tl::enums::RpcError::RpcError(rpc_error) = *rpc_error;

                Err(Error::RpcError {
                    code: rpc_error.error_code,
                    message: rpc_error.error_message,
                })
            }
            Err(_) => self.process_rpc_unknown::<F>(id),
        }
    }

    fn process_rpc_unknown<F>(&self, id: i32) -> Result<F::Result, Error>
    where
        F: Identify + Call,
    {
        Err(Error::ResponseMismatch {
            function: F::ID,
            response: id,
        })
    }
}
