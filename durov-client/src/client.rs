pub mod auth;
pub mod connect;
pub mod rpc;
pub mod updates;

use crate::config::Config;
use durov_mtclient::encrypted::EncryptedClient;
use std::marker::PhantomData;
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Clone)]
pub struct Client<T, S> {
    config: Arc<Config>,
    session: Arc<S>,
    client: Arc<RwLock<EncryptedClient>>,
    transport: PhantomData<T>,
}
