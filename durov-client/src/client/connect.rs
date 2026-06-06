use crate::client::updates::updater::Updater;
use crate::client::Client;
use crate::config::Config;
use crate::manager::Manager;
use crate::sessions::Session;
use crate::Error;
use durov_mtproto::transports::Transport;
use std::sync::Arc;
use tokio::sync::Mutex;

impl<T: Transport, S: Session> Client<T, S>
where
    T: Send + 'static,
{
    pub async fn connect(info: &str, config: Config) -> Result<Self, Error> {
        let session = S::connect(info).await?;
        session.init().await?;

        let config = Arc::new(config);
        let session = Arc::new(session);
        let manager = Manager::new(Arc::clone(&config), Arc::clone(&session));
        let clients = Arc::new(manager);
        let updater = Arc::new(Mutex::new(Updater::new()));

        Ok(Self { config, session, clients, updater })
    }

    pub async fn switch_dc(&self, dc_id: i32) -> Result<(), Error> {
        self.clients.switch(dc_id).await?;

        Ok(())
    }
}
