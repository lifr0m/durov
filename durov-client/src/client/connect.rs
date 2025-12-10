use crate::client::Client;
use crate::datacenters::{get_default_dc, get_public_key};
use crate::{tl, Config, Error};
use durov_mtclient::encrypted::EncryptedClient;
use durov_mtclient::plain::PlainClient;
use durov_mtclient::MtConfig;
use durov_mtproto::datacenter::{Datacenter, DatacenterType};
use durov_mtproto::transports::Transport;
use std::marker::PhantomData;

pub enum Auth {
    Absent {
        dc_type: DatacenterType,
        dc_id: Option<i32>,
    },
    Present {
        dc_type: DatacenterType,
        dc_id: i32,
        auth_key: Box<[u8; 256]>,
    },
}

impl<T: Transport> Client<T>
where
    T: Send + 'static,
{
    pub async fn connect(config: Config, auth: Auth) -> Result<Self, Error> {
        let dc_type = match auth {
            Auth::Absent { dc_type, .. } => dc_type,
            Auth::Present { dc_type, .. } => dc_type,
        };
        let client = connect::<T>(&config, auth).await?;

        Ok(Self { config, dc_type, client, transport: PhantomData })
    }

    pub async fn switch_dc(&mut self, dc_id: i32) -> Result<(), Error> {
        let auth = Auth::Absent {
            dc_type: self.dc_type,
            dc_id: Some(dc_id),
        };
        self.client = connect::<T>(&self.config, auth).await?;

        Ok(())
    }
}

async fn connect<T>(config: &Config, auth: Auth) -> Result<EncryptedClient, Error>
where
    T: Transport + Send + 'static,
{
    let client = match auth {
        Auth::Absent { dc_type, dc_id } => {
            if let Some(dc_id) = dc_id {
                let client = fresh_connect::<T>(config, get_default_dc(dc_type)).await?;
                let dc = pick_dc(&client, config, dc_type, dc_id).await?;
                fresh_connect::<T>(config, dc).await?
            } else {
                fresh_connect::<T>(config, get_default_dc(dc_type)).await?
            }
        }
        Auth::Present { dc_type, dc_id, auth_key } => {
            let client = fresh_connect::<T>(config, get_default_dc(dc_type)).await?;
            let dc = pick_dc(&client, config, dc_type, dc_id).await?;
            authed_connect::<T>(config, dc, *auth_key).await?
        }
    };
    init_connection(&client, config).await?;

    Ok(client)
}

async fn fresh_connect<T>(config: &Config, dc: Datacenter) -> Result<EncryptedClient, Error>
where
    T: Transport + Send + 'static,
{
    let mt_config = MtConfig {
        dc,
        use_gzip: config.use_compression,
    };
    let client = PlainClient::<T>::connect(mt_config).await?;
    let (client, auth_key) = client.auth().await?;
    println!("auth key: {auth_key:?}");
    Ok(client)
}

async fn authed_connect<T>(config: &Config, dc: Datacenter, auth_key: [u8; 256])
    -> Result<EncryptedClient, Error>
where
    T: Transport + Send + 'static,
{
    let mt_config = MtConfig {
        dc,
        use_gzip: config.use_compression,
    };
    Ok(EncryptedClient::connect::<T>(mt_config, auth_key).await?)
}

async fn pick_dc(client: &EncryptedClient, config: &Config, dc_type: DatacenterType, dc_id: i32)
    -> Result<Datacenter, Error>
{
    let config = init_connection(client, config).await?;
    let tl::enums::Config::Config(config) = config;

    Ok(config.dc_options.into_iter()
        .find_map(|option| {
            let tl::enums::DcOption::DcOption(option) = option;

            if !option.ipv6
                && !option.media_only
                && !option.tcpo_only
                && !option.cdn
                && !option.static_
                && option.id == dc_id
            {
                Some(Datacenter {
                    id: option.id,
                    typ: dc_type,
                    host: option.ip_address,
                    port: option.port as u16,
                    pubkey: get_public_key(dc_type),
                })
            } else {
                None
            }
        })
        .expect("can't find suitable dc"))
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
    }).await?)
}
