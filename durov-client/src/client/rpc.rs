use crate::client::Client;
use crate::sessions::Session;
use crate::{tl, Error};
use durov_mtproto::transports::Transport;
use durov_tl_types::deserialize::Deserialize;
use durov_tl_types::serialize::Serialize;
use durov_tl_types::{Call, Identify};
use std::sync::Arc;

impl<T: Transport, S: Session> Client<T, S>
where
    T: Send + 'static,
{
    pub async fn call<F>(&self, func: F) -> Result<F::Result, Error>
    where
        F: Identify + Call + Serialize + Send + Sync + 'static,
        F::Result: Deserialize + Send,
    {
        if self.config.updates {
            self.call_impl(func).await
        } else {
            self.call_impl(tl::functions::InvokeWithoutUpdates {
                query: func,
            }).await
        }
    }

    async fn call_impl<F>(&self, func: F) -> Result<F::Result, Error>
    where
        F: Identify + Call + Serialize + Send + Sync + 'static,
        F::Result: Deserialize + Send,
    {
        let func = Arc::new(func);

        loop {
            match self.client.read().await
                .call(Arc::clone(&func)).await
                .map_err(Error::from)
            {
                Ok(result) => break Ok(result),
                Err(Error::MtClient(durov_mtclient::Error::Resend)) => (),
                Err(err) => break Err(err),
            }
        }
    }
}
