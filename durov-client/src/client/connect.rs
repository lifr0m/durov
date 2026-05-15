use crate::client::updates::updater::Updater;
use crate::client::Client;
use crate::config::Config;
use crate::datacenters::{get_default_dc, get_public_key};
use crate::sessions::auth::Auth;
use crate::sessions::Session;
use crate::{tl, Error};
use durov_crypto::Datacenter;
use durov_mtclient::config::MtConfig;
use durov_mtclient::encrypted::EncryptedClient;
use durov_mtclient::plain::PlainClient;
use durov_mtproto::transports::Transport;
use std::sync::Arc;
use std::thread;
use tokio::sync::{Mutex, RwLock};

impl<T: Transport, S: Session> Client<T, S>
where
    T: Send + 'static,
{
    pub async fn connect(info: &str, config: Config) -> Result<Self, Error> {
        let session = S::connect(info).await?;
        session.init().await?;

        let client = if let Some(auth) = session.get_auth().await? {
            connect_auth(&config, auth).await?
        } else {
            let (client, auth) = connect_new(&config, None).await?;
            session.set_auth(&auth).await?;
            client
        };

        Ok(Self {
            config: Arc::new(config),
            session: Arc::new(session),
            client: Arc::new(RwLock::new(client)),
            updater: Arc::new(Mutex::new(Updater::new())),
        })
    }

    pub async fn switch_dc(&self, dc_id: i32) -> Result<(), Error> {
        let mut client = self.client.write().await;

        let auth;
        (*client, auth) = connect_new(&self.config, Some(dc_id)).await?;
        self.session.set_auth(&auth).await?;

        Ok(())
    }
}

async fn connect_new<T>(config: &Config, dc_id: Option<i32>)
    -> Result<(EncryptedClient<T>, Auth), Error>
where
    T: Transport + Send + 'static,
{
    let dc = get_dc::<T>(config, dc_id).await?;

    let (client, auth_key) = fresh_connect(dc.clone(), config).await?;
    init_connection(&client, config).await?;

    let auth = Auth {
        dc_id: dc.id,
        dc_host: dc.host,
        dc_port: dc.port,
        auth_key,
    };
    Ok((client, auth))
}

async fn connect_auth<T>(config: &Config, auth: Auth) -> Result<EncryptedClient<T>, Error>
where
    T: Transport + Send + 'static,
{
    let dc = Datacenter {
        id: auth.dc_id,
        prod: config.prod_dc,
        host: auth.dc_host,
        port: auth.dc_port,
        pubkey: get_public_key(config.prod_dc),
    };

    let client = authed_connect(dc, auth.auth_key, config).await?;
    init_connection(&client, config).await?;

    Ok(client)
}

async fn get_dc<T>(config: &Config, dc_id: Option<i32>) -> Result<Datacenter, Error>
where
    T: Transport + Send + 'static,
{
    let dc = get_default_dc(config.prod_dc);
    let (client, _) = fresh_connect::<T>(dc, config).await?;
    let tl_config = init_connection(&client, config).await?;

    let dc_id = match dc_id {
        Some(dc_id) => dc_id,
        None => {
            let nearest = client.call(tl::functions::help::GetNearestDc {}).await?;
            let tl::enums::NearestDc::NearestDc(nearest) = nearest;
            nearest.nearest_dc
        }
    };

    Ok(select_dc(tl_config, dc_id, config.prod_dc).expect("can't find suitable dc"))
}

async fn fresh_connect<T>(dc: Datacenter, config: &Config)
    -> Result<(EncryptedClient<T>, [u8; 256]), Error>
where
    T: Transport + Send + 'static,
{
    let mt_config = create_mt_config(config, dc);
    let client = PlainClient::connect(mt_config).await?;
    Ok(client.auth().await?)
}

async fn authed_connect<T>(dc: Datacenter, auth_key: [u8; 256], config: &Config)
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

fn select_dc(config: tl::enums::Config, id: i32, prod: bool) -> Option<Datacenter> {
    let tl::enums::Config::Config(config) = config;

    config.dc_options.into_iter()
        .find_map(|option| {
            let tl::enums::DcOption::DcOption(option) = option;

            (
                !option.ipv6
                    && !option.media_only
                    && !option.tcpo_only
                    && !option.cdn
                    && option.static_
                    && option.id == id
            ).then(|| Datacenter {
                id: option.id,
                prod,
                host: option.ip_address,
                port: option.port as u16,
                pubkey: get_public_key(prod),
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
