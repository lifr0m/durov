use crate::config::Config;
use crate::datacenters::{static_dc, PUBLIC_KEY};
use crate::sessions::auth::Auth;
use crate::sessions::Session;
use crate::{tl, Error};
use durov_crypto::Datacenter;
use durov_mtclient::config::MtConfig;
use durov_mtclient::encrypted::EncryptedClient;
use durov_mtclient::plain::PlainClient;
use durov_mtproto::transports::Transport;
use parking_lot::Mutex;
use std::collections::HashMap;
use std::sync::Arc;
use std::thread;
use tokio::sync::{Mutex as AsyncMutex, RwLock};

const DEFAULT_DC: i32 = 2;

pub enum DatacenterKey {
    Main,
    Concrete(i32),
}

pub struct Manager<T, S> {
    config: Arc<Config>,
    session: Arc<S>,
    main_dc: RwLock<i32>,
    clients: Mutex<HashMap<i32, Arc<AsyncMutex<Option<Arc<EncryptedClient<T>>>>>>>,
}

impl<T: Transport, S: Session> Manager<T, S>
where
    T: Send + 'static,
{
    pub async fn create(config: Arc<Config>, session: Arc<S>) -> Result<Self, Error> {
        let main_dc = session.list_auths().await?
            .iter()
            .find_map(|auth| auth.main.then_some(auth.dc_id))
            .unwrap_or(DEFAULT_DC);

        Ok(Self {
            config,
            session,
            main_dc: RwLock::new(main_dc),
            clients: Mutex::new(HashMap::new()),
        })
    }

    pub async fn get(&self, key: DatacenterKey) -> Result<Arc<EncryptedClient<T>>, Error> {
        let main_dc = self.main_dc.read().await;

        let dc_id = match key {
            DatacenterKey::Main => *main_dc,
            DatacenterKey::Concrete(dc_id) => dc_id,
        };

        let mut guard = self.lock(dc_id).await;

        if let Some(client) = self.get_from_cache(&guard) {
            return Ok(client);
        }

        if let Some(client) = self.get_from_session(dc_id).await? {
            self.set_to_cache(&mut guard, Arc::clone(&client));

            return Ok(client);
        }

        let (client, auth) = self.create_client(dc_id, *main_dc).await?;

        if dc_id != *main_dc {
            self.authorize_client(&client, dc_id).await?;
        }

        self.set_to_session(&auth).await?;
        self.set_to_cache(&mut guard, Arc::clone(&client));

        Ok(client)
    }

    pub async fn switch(&self, dc_id: i32) -> Result<(), Error> {
        let mut main_dc = self.main_dc.write().await;

        self.clients.lock()
            .remove(&main_dc);
        self.clients.lock()
            .remove(&dc_id);

        self.session.del_auth(*main_dc).await?;
        self.session.del_auth(dc_id).await?;

        *main_dc = dc_id;

        Ok(())
    }

    async fn lock(&self, dc_id: i32) -> tokio::sync::OwnedMutexGuard<Option<Arc<EncryptedClient<T>>>> {
        let lock = {
            let mut map = self.clients.lock();
            let lock = map.entry(dc_id).or_default();
            Arc::clone(lock)
        };
        lock.lock_owned().await
    }

    fn get_from_cache(&self, guard: &Option<Arc<EncryptedClient<T>>>) -> Option<Arc<EncryptedClient<T>>> {
        guard.as_ref()
            .map(Arc::clone)
    }

    fn set_to_cache(&self, guard: &mut Option<Arc<EncryptedClient<T>>>, client: Arc<EncryptedClient<T>>) {
        *guard = Some(client);
    }

    async fn get_from_session(&self, dc_id: i32) -> Result<Option<Arc<EncryptedClient<T>>>, Error> {
        let auth = self.session.list_auths().await?
            .into_iter()
            .find(|auth| auth.dc_id == dc_id);

        match auth {
            None => Ok(None),
            Some(auth) => {
                let client = auth_client(&self.config, auth).await?;
                let client = Arc::new(client);
                Ok(Some(client))
            }
        }
    }

    async fn set_to_session(&self, auth: &Auth) -> Result<(), Error> {
        self.session.set_auth(auth).await?;

        Ok(())
    }

    async fn create_client(&self, dc_id: i32, main_dc: i32) -> Result<(Arc<EncryptedClient<T>>, Auth), Error> {
        let main = dc_id == main_dc;
        let (client, auth) = fresh_client(&self.config, dc_id, main).await?;
        let client = Arc::new(client);

        Ok((client, auth))
    }

    async fn authorize_client(&self, client: &EncryptedClient<T>, dc_id: i32) -> Result<(), Error> {
        let main = Box::pin(self.get(DatacenterKey::Main)).await?;

        let authorization = main.call(tl::functions::auth::ExportAuthorization { dc_id }).await?;
        let tl::enums::auth::ExportedAuthorization::ExportedAuthorization(authorization) = authorization;

        client.call(tl::functions::auth::ImportAuthorization {
            id: authorization.id,
            bytes: authorization.bytes,
        }).await?;

        Ok(())
    }
}

async fn fresh_client<T>(config: &Config, dc_id: i32, main: bool) -> Result<(EncryptedClient<T>, Auth), Error>
where
    T: Transport + Send + 'static,
{
    let dc = static_dc(dc_id);
    let (client, auth_key) = connect_fresh(config, dc.clone()).await?;
    init_connection(config, &client).await?;

    let auth = Auth {
        dc_id: dc.id,
        dc_host: dc.host,
        dc_port: dc.port,
        auth_key,
        main,
    };
    Ok((client, auth))
}

async fn auth_client<T>(config: &Config, auth: Auth) -> Result<EncryptedClient<T>, Error>
where
    T: Transport + Send + 'static,
{
    let dc = Datacenter {
        id: auth.dc_id,
        host: auth.dc_host,
        port: auth.dc_port,
        pubkey: PUBLIC_KEY,
    };

    let client = connect_auth(config, dc, auth.auth_key).await?;
    init_connection(config, &client).await?;

    Ok(client)
}

async fn connect_fresh<T>(config: &Config, dc: Datacenter) -> Result<(EncryptedClient<T>, [u8; 256]), Error>
where
    T: Transport + Send + 'static,
{
    let mt_config = create_mt_config(config, dc);
    let client = PlainClient::connect(mt_config).await?;
    Ok(client.auth().await?)
}

async fn connect_auth<T>(config: &Config, dc: Datacenter, auth_key: [u8; 256]) -> Result<EncryptedClient<T>, Error>
where
    T: Transport + Send + 'static,
{
    let mt_config = create_mt_config(config, dc);
    Ok(EncryptedClient::connect(mt_config, auth_key).await?)
}

async fn init_connection<T>(config: &Config, client: &EncryptedClient<T>) -> Result<tl::enums::Config, Error>
where
    T: Transport + Send + 'static,
{
    Ok(client.call(tl::functions::InvokeWithLayer {
        layer: tl::LAYER,
        query: tl::functions::InitConnection {
            api_id: config.api_id,
            device_model: config.device_model.clone(),
            system_version: config.system_version.clone(),
            app_version: config.app_version.clone(),
            system_lang_code: config.system_lang_code.clone(),
            lang_pack: config.lang_pack.clone(),
            lang_code: config.lang_code.clone(),
            proxy: None,
            params: config.params.clone(),
            query: tl::functions::help::GetConfig {},
        },
    }).await?)
}

fn create_mt_config(config: &Config, dc: Datacenter) -> MtConfig {
    MtConfig {
        dc,
        proxy: config.proxy.clone(),
        use_gzip: config.use_compression,
        updates: config.updates,
        parallelism: if config.high_load {
            thread::available_parallelism()
                .unwrap()
                .get()
        } else {
            1
        },
    }
}
