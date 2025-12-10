pub mod auth;
pub mod connect;

use crate::{Config, Error};
use durov_mtclient::encrypted::EncryptedClient;
use durov_mtproto::datacenter::DatacenterType;
use durov_tl_types::deserialize::Deserialize;
use durov_tl_types::serialize::Serialize;
use durov_tl_types::{Call, Identify};
use std::marker::PhantomData;

pub struct Client<T> {
    config: Config,
    dc_type: DatacenterType,
    client: EncryptedClient,
    transport: PhantomData<T>,
}

impl<T> Client<T> {
    pub async fn call<F>(&self, func: F) -> Result<F::Result, Error>
    where
        F: Identify + Call + Serialize + Send + 'static,
        F::Result: Deserialize + Send,
    {
        Ok(self.client.call(func).await?)
    }
}
