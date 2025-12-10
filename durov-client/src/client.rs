mod auth;
mod connect;

use crate::{Config, Error};
use durov_mtclient::encrypted::EncryptedClient;
use durov_mtproto::datacenter::DatacenterType;
use durov_mtproto::transports::Transport;
use durov_tl_types::deserialize::Deserialize;
use durov_tl_types::serialize::Serialize;
use durov_tl_types::{Call, Identify};
use std::marker::PhantomData;

pub enum Auth {
    Absent {
        dc_type: DatacenterType,
        dc_id: Option<i32>,
    },
    Present {
        dc_type: DatacenterType,
        dc_id: i32,
        auth_key: Box<[u8; 256]>,
    },
}

pub struct Client<T> {
    config: Config,
    dc_type: DatacenterType,
    client: EncryptedClient,
    transport: PhantomData<T>,
}

impl<T: Transport> Client<T>
where
    T: Send + 'static,
{
    pub async fn connect(config: Config, auth: Auth) -> Result<Self, Error> {
        let dc_type = match auth {
            Auth::Absent { dc_type, .. } => dc_type,
            Auth::Present { dc_type, .. } => dc_type,
        };
        let client = Self::connect_inner(&config, auth).await?;

        Ok(Self { config, dc_type, client, transport: PhantomData })
    }
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
