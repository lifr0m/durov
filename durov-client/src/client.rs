pub mod auth;
pub mod connect;
pub mod rpc;
pub mod updates;
pub mod resolve;

use crate::client::updates::updater::Updater;
use crate::config::Config;
use crate::manager::Manager;
use std::sync::Arc;
use tokio::sync::Mutex;

pub struct Client<T, S> {
    config: Arc<Config>,
    session: Arc<S>,
    clients: Arc<Manager<T, S>>,
    updater: Arc<Mutex<Updater>>,
}

impl<T, S> Clone for Client<T, S> {
    fn clone(&self) -> Self {
        Self {
            config: Arc::clone(&self.config),
            session: Arc::clone(&self.session),
            clients: Arc::clone(&self.clients),
            updater: Arc::clone(&self.updater),
        }
    }
}
