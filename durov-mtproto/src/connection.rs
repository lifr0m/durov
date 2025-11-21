use crate::protocols::encrypted::Encrypted;
use crate::protocols::plain::Plain;
use crate::protocols::Protocol;
use crate::transports::full::Full;
use crate::transports::Transport;
use crate::{auth, crypto};
use durov_tl_types::deserialize::Deserialize;
use durov_tl_types::serialize::Serialize;
use durov_tl_types::Call;
use thiserror::Error;
use tokio::io;
use tokio::net::TcpStream;

#[derive(Error, Debug)]
pub enum Error {
    #[error("io: {0}")]
    Io(#[from] io::Error),

    #[error("protocol: {0}")]
    Protocol(#[from] crate::protocols::Error),

    #[error("transport: {0}")]
    Transport(#[from] crate::transports::Error),

    #[error("deserialize: {0}")]
    Deserialize(#[from] durov_tl_types::deserialize::Error),

    #[error("auth: {0}")]
    Auth(#[from] auth::Error),

    #[error("exceeded attempts trying auth step 4")]
    AuthStep4Failed,

    #[error("exceeded attempts trying auth")]
    AuthFailed,
}

#[derive(Copy, Clone)]
pub enum DcType {
    Production,
    Test,
    Media,
}

pub struct ConnectionConfig {
    pub dc: i32,
    pub dc_type: DcType,
    pub server_pubkey: rsa::RsaPublicKey,
}

pub struct Connection<T, P> {
    transport: T,
    protocol: P,
    config: ConnectionConfig,
}

impl<T: Transport, P: Protocol> Connection<T, P> {
    fn new(transport: T, protocol: P, config: ConnectionConfig) -> Self {
        Self { transport, protocol, config }
    }

    pub async fn call<F: Call + Serialize>(&mut self, func: &F) -> Result<F::Result, Error> {
        let data = func.to_bytes();
        let data = self.protocol.pack(&data)?;
        self.transport.send(&data).await?;

        let data = self.transport.receive().await?;
        // todo: handle "ignore this message"
        let data = self.protocol.unpack(&data)?;
        Ok(F::Result::from_bytes(&data)?)
    }
}

impl<P: Protocol> Connection<Full<TcpStream>, P> {
    pub async fn connect(
        host: &str,
        port: u16,
        protocol: P,
        config: ConnectionConfig,
    ) -> Result<Self, Error> {
        let stream = TcpStream::connect((host, port)).await?;
        let transport = Full::new(stream);
        Ok(Self::new(transport, protocol, config))
    }
}

impl<T: Transport> Connection<T, Plain> {
    pub async fn auth(mut self) -> Result<Connection<T, Encrypted>, Error> {
        for auth_attempt in 1..=3 {
            log::info!("trying auth (attempt {auth_attempt})");

            let step1 = auth::step1();
            let res = self.call(&step1.req).await?;

            let step2 = auth::step2(
                res,
                step1.nonce,
                self.config.dc,
                self.config.dc_type,
                &self.config.server_pubkey,
            )?;
            let res = match self.call(&step2.req).await {
                Ok(res) => res,
                Err(Error::Transport(crate::transports::Error::Application(404))) => {
                    log::warn!("received error 404 during auth");
                    continue;
                }
                Err(err) => return Err(err),
            };

            let step3 = match auth::step3(
                res,
                step1.nonce,
                step2.server_nonce,
                step2.new_nonce,
            ) {
                Ok(step3) => step3,
                Err(auth::Error::Restart) => {
                    log::warn!("auth error requires handshake restart");
                    continue;
                }
                Err(err) => return Err(err.into()),
            };
            self.protocol.set_server_time(step3.server_time as f64);

            let mut prev_auth_key_aux_id = None;
            for step4_attempt in 1..=3 {
                log::info!("trying step 4 (attempt {step4_attempt})");

                let step4 = auth::step4(
                    step1.nonce,
                    step2.server_nonce,
                    step3.tmp_aes_key,
                    step3.tmp_aes_iv,
                    step3.p,
                    &step3.g,
                    &step3.g_a,
                    prev_auth_key_aux_id,
                )?;
                let res = self.call(&step4.req).await?;

                let step5 = match auth::step5(
                    res,
                    step1.nonce,
                    step2.server_nonce,
                    step2.new_nonce,
                    &step4.auth_key,
                ) {
                    Ok(step5) => step5,
                    Err(auth::Error::RetryStep4) => {
                        log::warn!("auth error requires retry starting from step 4");
                        prev_auth_key_aux_id = Some(crypto::compute_auth_key_aux_id(&step4.auth_key));
                        continue;
                    }
                    Err(err) => return Err(err.into()),
                };

                return Ok(self.upgrade(step4.auth_key, step5.server_salt));
            }

            return Err(Error::AuthStep4Failed);
        }

        Err(Error::AuthFailed)
    }

    fn upgrade(self, auth_key: [u8; 256], salt: [u8; 8]) -> Connection<T, Encrypted> {
        let protocol = Encrypted::from_plain(self.protocol, auth_key, salt);
        Connection::new(self.transport, protocol, self.config)
    }
}

impl<T: Transport> Connection<T, Encrypted> {
    pub fn auth_key(&self) -> &[u8] {
        self.protocol.auth_key()
    }
}
