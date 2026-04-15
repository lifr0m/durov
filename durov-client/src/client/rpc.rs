use crate::client::Client;
use crate::sessions::Session;
use crate::{tl, Error};
use durov_mtproto::transports::Transport;
use durov_tl_types::deserialize::Deserialize;
use durov_tl_types::serialize::Serialize;
use durov_tl_types::{Call, Identify};

impl<T: Transport, S: Session> Client<T, S>
where
    T: Send + 'static,
{
    pub async fn call<F>(&self, func: F) -> Result<F::Result, Error>
    where
        F: Identify + Call + Serialize + Send + 'static,
        F::Result: Deserialize + Send,
    {
        let client = self.client.read().await;

        if self.config.updates {
            Ok(client.call(func).await?)
        } else {
            Ok(client.call(tl::functions::InvokeWithoutUpdates {
                query: func,
            }).await?)
        }
    }
}
