use crate::config::Config;
use crate::datacenters::{static_dc, PUBLIC_KEY};
use crate::sessions::Session;
use crate::{tl, Error};
use durov_crypto::Datacenter;
use durov_mtclient::config::MtConfig;
use durov_mtclient::encrypted::EncryptedClient;
use durov_mtclient::plain::PlainClient;
use durov_mtproto::transports::Transport;
use parking_lot::Mutex;
use std::collections::HashMap;
use std::hash::Hash;
use std::sync::Arc;
use tokio::sync::{Mutex as AsyncMutex, RwLock};

const DEFAULT_DC: i32 = 2;

#[derive(Copy, Clone, Eq, PartialEq, Hash)]
pub enum ClientKey {
    Main,
    Upload {
        conn_id: i32,
    },
    Download {
        dc_id: i32,
        conn_id: i32,
    },
}

pub struct Manager<T, S> {
    config: Arc<Config>,
    session: Arc<S>,
    main_lock: RwLock<()>,
    client_map: Mutex<HashMap<ClientKey, Arc<EncryptedClient<T>>>>,
    client_locks: Mutex<HashMap<ClientKey, Arc<AsyncMutex<()>>>>,
    dc_locks: Mutex<HashMap<i32, Arc<AsyncMutex<()>>>>,
}

impl<T: Transport, S: Session> Manager<T, S>
where
    T: Send + 'static,
{
    pub fn new(config: Arc<Config>, session: Arc<S>) -> Self {
        Self {
            config,
            session,
            main_lock: RwLock::new(()),
            client_map: Mutex::new(HashMap::new()),
            client_locks: Mutex::new(HashMap::new()),
            dc_locks: Mutex::new(HashMap::new()),
        }
    }

    pub async fn get(&self, key: ClientKey) -> Result<Arc<EncryptedClient<T>>, Error> {
        let _lock = self.main_lock.read().await;

        self.get_inner(key).await
    }

    pub async fn switch(&self, dc_id: i32) -> Result<(), Error> {
        let _lock = self.main_lock.write().await;

        self.client_map.lock()
            .retain(|key, _| matches!(key, ClientKey::Download { .. }));
        self.session.set_main_dc(dc_id).await?;

        Ok(())
    }

    async fn get_inner(&self, key: ClientKey) -> Result<Arc<EncryptedClient<T>>, Error> {
        let _lock = Self::lock(&self.client_locks, key).await;

        if let Some(client) = self.get_from_cache(key) {
            return Ok(client);
        }

        let main_dc = self.session.get_main_dc().await?
            .unwrap_or(DEFAULT_DC);

        let (dc_id, media) = match key {
            ClientKey::Main => (main_dc, false),
            ClientKey::Upload { .. } => (main_dc, true),
            ClientKey::Download { dc_id, .. } => (dc_id, true),
        };

        let auth_key = self.get_auth_key(dc_id, main_dc).await?;

        let dc = self.get_datacenter(dc_id, media).await?;
        let client = auth_client(&self.config, dc, auth_key).await?;
        let client = Arc::new(client);

        self.set_to_cache(key, Arc::clone(&client));

        Ok(client)
    }

    fn get_from_cache(&self, key: ClientKey) -> Option<Arc<EncryptedClient<T>>> {
        self.client_map.lock()
            .get(&key)
            .map(Arc::clone)
    }

    fn set_to_cache(&self, key: ClientKey, client: Arc<EncryptedClient<T>>) {
        self.client_map.lock()
            .insert(key, client);
    }

    async fn get_auth_key(&self, dc_id: i32, main_dc: i32) -> Result<[u8; 256], Error> {
        let _lock = Self::lock(&self.dc_locks, dc_id).await;

        if let Some(auth_key) = self.session.get_auth_key(dc_id).await? {
            return Ok(auth_key);
        }

        let dc = self.get_datacenter(dc_id, false).await?;
        let (client, auth_key) = fresh_client(&self.config, dc).await?;

        if dc_id != main_dc {
            self.authorize(&client, dc_id).await?;
        }

        self.session.set_auth_key(dc_id, auth_key).await?;

        Ok(auth_key)
    }

    async fn lock<K>(map: &Mutex<HashMap<K, Arc<AsyncMutex<()>>>>, key: K) -> tokio::sync::OwnedMutexGuard<()>
    where
        K: Eq + Hash,
    {
        let lock = {
            let mut map = map.lock();
            let lock = map.entry(key).or_default();
            Arc::clone(lock)
        };
        lock.lock_owned().await
    }

    async fn authorize(&self, client: &EncryptedClient<T>, dc_id: i32) -> Result<(), Error> {
        let main = Box::pin(self.get_inner(ClientKey::Main)).await?;

        let authorization = main.call(tl::functions::auth::ExportAuthorization { dc_id }).await?;
        let tl::enums::auth::ExportedAuthorization::ExportedAuthorization(authorization) = authorization;

        client.call(tl::functions::auth::ImportAuthorization {
            id: authorization.id,
            bytes: authorization.bytes,
        }).await?;

        Ok(())
    }

    async fn get_datacenter(&self, dc_id: i32, media: bool) -> Result<Datacenter, Error> {
        Ok(if media {
            let main = Box::pin(self.get_inner(ClientKey::Main)).await?;
            let config = main.call(tl::functions::help::GetConfig {}).await?;
            find_media_dc(config, dc_id)
                .expect("cannot find suitable media dc")
        } else {
            static_dc(dc_id)
        })
    }
}

fn find_media_dc(config: tl::enums::Config, dc_id: i32) -> Option<Datacenter> {
    let tl::enums::Config::Config(config) = config;

    let select = |option: &tl::enums::DcOption| {
        let tl::enums::DcOption::DcOption(option) = option;

        if !option.ipv6 && !option.tcpo_only && !option.cdn && option.id == dc_id {
            Some(Datacenter {
                id: option.id,
                host: option.ip_address.clone(),
                port: option.port as u16,
                pubkey: PUBLIC_KEY,
            })
        } else {
            None
        }
    };

    if let Some(dc) = config.dc_options.iter()
        .filter(|option| {
            let tl::enums::DcOption::DcOption(option) = option;

            option.media_only
        })
        .find_map(select)
    {
        return Some(dc);
    }

    config.dc_options.iter()
        .find_map(select)
}

async fn fresh_client<T>(config: &Config, dc: Datacenter) -> Result<(EncryptedClient<T>, [u8; 256]), Error>
where
    T: Transport + Send + 'static,
{
    let (client, auth_key) = connect_fresh(config, dc).await?;
    init_connection(&client, config).await?;

    Ok((client, auth_key))
}

async fn auth_client<T>(config: &Config, dc: Datacenter, auth_key: [u8; 256]) -> Result<EncryptedClient<T>, Error>
where
    T: Transport + Send + 'static,
{
    let client = connect_auth(config, dc, auth_key).await?;
    init_connection(&client, config).await?;

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

async fn init_connection<T>(client: &EncryptedClient<T>, config: &Config) -> Result<(), Error>
where
    T: Transport + Send + 'static,
{
    client.call(tl::functions::InvokeWithLayer {
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
    }).await?;

    Ok(())
}

fn create_mt_config(config: &Config, dc: Datacenter) -> MtConfig {
    MtConfig {
        dc,
        proxy: config.proxy.clone(),
        use_gzip: config.use_compression,
        updates: config.updates,
    }
}
