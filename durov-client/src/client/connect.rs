use crate::client::Client;
use crate::config::Config;
use crate::datacenters::{get_default_dc, get_public_key};
use crate::sessions::{Auth, Session};
use crate::{tl, Error};
use durov_mtclient::config::MtConfig;
use durov_mtclient::encrypted::EncryptedClient;
use durov_mtclient::plain::PlainClient;
use durov_mtproto::datacenter::Datacenter;
use durov_mtproto::transports::Transport;
use std::marker::PhantomData;
use std::sync::Arc;
use tokio::sync::RwLock;

impl<T: Transport, S: Session> Client<T, S>
where
    T: Send + 'static,
{
    pub async fn connect(info: &str, config: Config) -> Result<Self, Error> {
        let session = S::connect(info).await?;
        session.init().await?;

        let client = if let Some(auth) = session.get_auth().await? {
            connect_auth::<T>(&config, auth).await?
        } else {
            let (client, auth) = connect_new::<T>(&config, None).await?;
            session.set_auth(&auth).await?;
            client
        };

        Ok(Self {
            config: Arc::new(config),
            session: Arc::new(session),
            client: Arc::new(RwLock::new(client)),
            transport: PhantomData,
        })
    }

    pub async fn switch_dc(&self, dc_id: i32, migrate: bool) -> Result<(), Error> {
        // If client is already locked for write it means switching is happening right now.
        // We need to just wait until it's finished. It happens by locking client for read.
        let Ok(mut client) = self.client.try_write() else {
            return Ok(());
        };

        if migrate {
            let dc = get_dc::<T>(&self.config, Some(dc_id)).await?;

            let mut auth = self.session.get_auth().await?
                .expect("auth should be saved when connecting");
            auth.dc_id = dc.id;
            auth.dc_host = dc.host;
            auth.dc_port = dc.port;
            self.session.set_auth(&auth).await?;

            *client = connect_auth::<T>(&self.config, auth).await?;
        } else {
            let auth;
            (*client, auth) = connect_new::<T>(&self.config, Some(dc_id)).await?;
            self.session.set_auth(&auth).await?;
        }

        Ok(())
    }
}

async fn connect_new<T>(config: &Config, dc_id: Option<i32>)
    -> Result<(EncryptedClient, Auth), Error>
where
    T: Transport + Send + 'static,
{
    let dc = get_dc::<T>(config, dc_id).await?;
    let (client, auth_key) = fresh_connect::<T>(dc.clone(), config).await?;
    init_connection(&client, config).await?;

    let auth = Auth {
        dc_id: dc.id,
        dc_host: dc.host,
        dc_port: dc.port,
        auth_key,
    };
    Ok((client, auth))
}

async fn connect_auth<T>(config: &Config, auth: Auth) -> Result<EncryptedClient, Error>
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

    let client = authed_connect::<T>(dc, auth.auth_key, config).await?;
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
            let nearest = client.call(tl::functions::help::GetNearestDc {}.into()).await?;
            let tl::enums::NearestDc::NearestDc(nearest) = nearest;
            nearest.nearest_dc
        }
    };

    Ok(select_dc(tl_config, dc_id, config.prod_dc))
}

async fn fresh_connect<T>(dc: Datacenter, config: &Config)
    -> Result<(EncryptedClient, [u8; 256]), Error>
where
    T: Transport + Send + 'static,
{
    let mt_config = MtConfig { dc, use_gzip: config.use_compression };
    let client = PlainClient::<T>::connect(mt_config).await?;
    Ok(client.auth().await?)
}

async fn authed_connect<T>(dc: Datacenter, auth_key: [u8; 256], config: &Config)
    -> Result<EncryptedClient, Error>
where
    T: Transport + Send + 'static,
{
    let mt_config = MtConfig { dc, use_gzip: config.use_compression };
    Ok(EncryptedClient::connect::<T>(mt_config, auth_key).await?)
}

async fn init_connection(client: &EncryptedClient, config: &Config)
    -> Result<tl::enums::Config, Error>
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
    }.into()).await?)
}

fn select_dc(config: tl::enums::Config, id: i32, prod: bool) -> Datacenter {
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
            ).then_some(Datacenter {
                id: option.id,
                prod,
                host: option.ip_address,
                port: option.port as u16,
                pubkey: get_public_key(prod),
            })
        })
        .expect("cant find suitable dc")
}
