pub mod auth;
pub mod connect;
pub mod rpc;
pub mod updates;

use crate::client::updates::updater::Updater;
use crate::config::Config;
use durov_mtclient::encrypted::EncryptedClient;
use std::sync::Arc;
use tokio::sync::{Mutex, RwLock};

pub struct Client<T, S> {
    config: Arc<Config>,
    session: Arc<S>,
    client: Arc<RwLock<EncryptedClient<T>>>,
    updater: Arc<Mutex<Updater>>,
}

impl<T, S> Clone for Client<T, S> {
    fn clone(&self) -> Self {
        Self {
            config: Arc::clone(&self.config),
            session: Arc::clone(&self.session),
            client: Arc::clone(&self.client),
            updater: Arc::clone(&self.updater),
        }
    }
}
