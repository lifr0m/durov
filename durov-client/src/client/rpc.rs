use crate::client::Client;
use crate::sessions::Session;
use crate::Error;
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
        let func = Arc::new(func);

        loop {
            match self.client.read().await
                .call(Arc::clone(&func)).await
                .map_err(Error::from)
            {
                Ok(result) => break Ok(result),
                Err(Error::MtClient(durov_mtclient::Error::Resend)) => (),
                Err(err) if err.is(303, "USER_MIGRATE") => {
                    let dc_id = err.parse("USER_MIGRATE_X")?;
                    self.switch_dc(dc_id, true).await?;
                }
                Err(err) => break Err(err),
            }
        }
    }
}
