use crate::config::Config;
use crate::datacenters::{default_dc, PUBLIC_KEY};
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

#[derive(Copy, Clone, Eq, PartialEq, Hash)]
pub enum ClientKey {
    Main,
    Media(i32),
}

pub struct Manager<T, S> {
    config: Arc<Config>,
    session: Arc<S>,
    main_dc: Mutex<Option<i32>>,
    clients: Mutex<HashMap<ClientKey, Arc<EncryptedClient<T>>>>,
    locks: Mutex<HashMap<ClientKey, Arc<tokio::sync::Mutex<()>>>>,
}

impl<T: Transport, S: Session> Manager<T, S>
where
    T: Send + 'static,
{
    pub fn new(config: Arc<Config>, session: Arc<S>) -> Self {
        Self {
            config,
            session,
            main_dc: Mutex::new(None),
            clients: Mutex::new(HashMap::new()),
            locks: Mutex::new(HashMap::new()),
        }
    }

    pub async fn get(&self, key: ClientKey) -> Result<Arc<EncryptedClient<T>>, Error> {
        let _lock = self.lock(key).await;

        if let Some(client) = self.get_from_cache(key) {
            return Ok(client);
        }

        if let Some(client) = self.get_from_session(key).await? {
            self.set_to_cache(key, Arc::clone(&client));

            return Ok(client);
        }

        let (client, auth) = self.create(key).await?;

        if let ClientKey::Media(dc_id) = key {
            let main = Box::pin(self.get(ClientKey::Main)).await?;

            let exported = main.call(tl::functions::auth::ExportAuthorization { dc_id }).await?;
            let tl::enums::auth::ExportedAuthorization::ExportedAuthorization(exported) = exported;

            client.call(tl::functions::auth::ImportAuthorization {
                id: exported.id,
                bytes: exported.bytes,
            }).await?;
        }

        self.set_to_cache(key, Arc::clone(&client));
        self.set_to_session(&auth).await?;

        Ok(client)
    }

    pub async fn switch(&self, dc_id: i32) -> Result<(), Error> {
        let _lock = self.lock(ClientKey::Main).await;

        self.clients.lock()
            .remove(&ClientKey::Main);

        self.session.del_auth().await?;

        *self.main_dc.lock() = Some(dc_id);

        Ok(())
    }

    async fn lock(&self, key: ClientKey) -> tokio::sync::OwnedMutexGuard<()> {
        let lock = {
            let mut map = self.locks.lock();
            let lock = map.entry(key).or_default();
            Arc::clone(lock)
        };
        lock.lock_owned().await
    }

    fn get_from_cache(&self, key: ClientKey) -> Option<Arc<EncryptedClient<T>>> {
        self.clients.lock()
            .get(&key)
            .map(Arc::clone)
    }

    fn set_to_cache(&self, key: ClientKey, client: Arc<EncryptedClient<T>>) {
        self.clients.lock()
            .insert(key, client);
    }

    async fn get_from_session(&self, key: ClientKey) -> Result<Option<Arc<EncryptedClient<T>>>, Error> {
        let auth = self.session.list_auths().await?
            .into_iter()
            .find(|auth| match key {
                ClientKey::Main => !auth.media,
                ClientKey::Media(dc_id) => auth.dc_id == dc_id && auth.media,
            });

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

    async fn create(&self, key: ClientKey) -> Result<(Arc<EncryptedClient<T>>, Auth), Error> {
        let (dc_id, media) = match (key, *self.main_dc.lock()) {
            (ClientKey::Main, None) => (None, false),
            (ClientKey::Main, Some(dc_id)) => (Some(dc_id), false),
            (ClientKey::Media(dc_id), _) => (Some(dc_id), true),
        };

        let (client, auth) = fresh_client(&self.config, dc_id, media).await?;
        let client = Arc::new(client);

        Ok((client, auth))
    }
}

async fn fresh_client<T>(config: &Config, dc_id: Option<i32>, media: bool)
    -> Result<(EncryptedClient<T>, Auth), Error>
where
    T: Transport + Send + 'static,
{
    let dc = get_dc::<T>(config, dc_id, media).await?;

    let (client, auth_key) = connect_fresh(dc.clone(), config).await?;
    init_connection(&client, config).await?;

    let auth = Auth {
        dc_id: dc.id,
        dc_host: dc.host,
        dc_port: dc.port,
        auth_key,
        media,
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

    let client = connect_auth(dc, auth.auth_key, config).await?;
    init_connection(&client, config).await?;

    Ok(client)
}

async fn get_dc<T>(config: &Config, dc_id: Option<i32>, media: bool) -> Result<Datacenter, Error>
where
    T: Transport + Send + 'static,
{
    let (client, _) = connect_fresh::<T>(default_dc(), config).await?;
    let tl_config = init_connection(&client, config).await?;

    let dc_id = match dc_id {
        Some(dc_id) => dc_id,
        None => {
            let nearest = client.call(tl::functions::help::GetNearestDc {}).await?;
            let tl::enums::NearestDc::NearestDc(nearest) = nearest;
            nearest.nearest_dc
        }
    };

    Ok(
        select_dc(tl_config, dc_id, media)
            .expect("can't find suitable dc")
    )
}

async fn connect_fresh<T>(dc: Datacenter, config: &Config)
    -> Result<(EncryptedClient<T>, [u8; 256]), Error>
where
    T: Transport + Send + 'static,
{
    let mt_config = create_mt_config(config, dc);
    let client = PlainClient::connect(mt_config).await?;
    Ok(client.auth().await?)
}

async fn connect_auth<T>(dc: Datacenter, auth_key: [u8; 256], config: &Config)
    -> Result<EncryptedClient<T>, Error>
where
    T: Transport + Send + 'static,
{
    let mt_config = create_mt_config(config, dc);
    Ok(EncryptedClient::connect(mt_config, auth_key).await?)
}

async fn init_connection<T>(client: &EncryptedClient<T>, config: &Config)
    -> Result<tl::enums::Config, Error>
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

fn select_dc(config: tl::enums::Config, id: i32, media: bool) -> Option<Datacenter> {
    let tl::enums::Config::Config(mut config) = config;

    config.dc_options.sort_by_key(|option| {
        let tl::enums::DcOption::DcOption(option) = option;

        !option.media_only
    });

    config.dc_options.into_iter()
        .find_map(|option| {
            let tl::enums::DcOption::DcOption(option) = option;

            (
                !option.ipv6
                    && (media || !option.media_only)
                    && !option.tcpo_only
                    && !option.cdn
                    && option.static_
                    && option.id == id
            ).then_some(Datacenter {
                id: option.id,
                host: option.ip_address,
                port: option.port as u16,
                pubkey: PUBLIC_KEY,
            })
        })
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
