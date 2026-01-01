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
            let dc = Datacenter {
                id: auth.dc_id,
                prod: config.prod_dc,
                host: auth.dc_host,
                port: auth.dc_port,
                pubkey: get_public_key(config.prod_dc),
            };
            let client = authed_connect::<T>(dc, auth.auth_key, &config).await?;
            init_connection(&client, &config).await?;
            client
        } else {
            let dc = get_default_dc(config.prod_dc);
            let (client, _) = fresh_connect::<T>(dc, &config).await?;
            let tl_config = init_connection(&client, &config).await?;
            let nearest_dc = client.call(tl::functions::help::GetNearestDc {}.into()).await?;
            let tl::enums::NearestDc::NearestDc(nearest_dc) = nearest_dc;
            let client = new_connect::<T, _>(&session, &config, tl_config, nearest_dc.nearest_dc).await?;
            init_connection(&client, &config).await?;
            client
        };

        Ok(Self {
            config: Arc::new(config),
            session: Arc::new(session),
            client: Arc::new(RwLock::new(client)),
            transport: PhantomData,
        })
    }

    // todo: maybe move migrate?
    pub async fn switch_dc(&self, id: i32, migrate: bool) -> Result<(), Error> {
        // If client is already locked for write it means switching is happening right now.
        // We need to just wait until it's finished. It happens by locking client for read.
        let Ok(mut client) = self.client.try_write() else {
            return Ok(());
        };

        // Regarding user migration, docs state: "Once this happens, when executing any query
        // transmitted to the old DC, the API will return the USER_MIGRATE_X error".
        // I don't actually understand whether this applies to requests like help.getConfig
        // or auth.exportAuthorization, but logically it should because there are no
        // other way we can get authorized on new dc except sending sms code and login again.
        let authorization = if migrate {
            Some(client.call(tl::functions::auth::ExportAuthorization {
                dc_id: id,
            }.into()).await?)
        } else {
            None
        };

        let tl_config = client.call(tl::functions::help::GetConfig {}.into()).await?;
        *client = new_connect::<T, _>(self.session.as_ref(), &self.config, tl_config, id).await?;
        init_connection(&client, &self.config).await?;

        if let Some(authorization) = authorization {
            let tl::enums::auth::ExportedAuthorization::ExportedAuthorization(authorization) = authorization;

            client.call(tl::functions::auth::ImportAuthorization {
                id: authorization.id,
                bytes: authorization.bytes,
            }.into()).await?;
        }

        Ok(())
    }
}

async fn new_connect<T, S>(session: &S, config: &Config, tl_config: tl::enums::Config, dc_id: i32)
    -> Result<EncryptedClient, Error>
where
    T: Transport + Send + 'static,
    S: Session,
{
    let dc = select_dc(tl_config, dc_id, config.prod_dc);
    let (client, auth_key) = fresh_connect::<T>(dc.clone(), config).await?;
    session.set_auth(Auth {
        dc_id: dc.id,
        dc_host: dc.host,
        dc_port: dc.port,
        auth_key,
    }).await?;
    Ok(client)
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
            ).then(|| Datacenter {
                id: option.id,
                prod,
                host: option.ip_address,
                port: option.port as u16,
                pubkey: get_public_key(prod),
            })
        })
        .expect("cant find suitable dc")
}
