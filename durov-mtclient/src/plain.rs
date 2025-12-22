use crate::config::MtConfig;
use crate::encrypted::EncryptedClient;
use crate::{tcp, Error};
use durov_mtproto::auth;
use durov_mtproto::protocols::encrypted::Encrypted;
use durov_mtproto::protocols::plain::Plain;
use durov_mtproto::transports::Transport;
use durov_tl_types::buffer::Buffer;
use durov_tl_types::deserialize::Deserialize;
use durov_tl_types::serialize::Serialize;
use durov_tl_types::Call;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

pub struct PlainClient<T> {
    config: MtConfig,
    stream: TcpStream,
    transport: T,
    protocol: Plain,
}

impl<T: Transport> PlainClient<T> {
    pub async fn connect(config: MtConfig) -> Result<Self, Error> {
        let stream = tcp::connect(&config.dc.host, config.dc.port).await?;
        let transport = T::default();
        let protocol = Plain::new();
        Ok(Self { config, stream, transport, protocol })
    }

    pub async fn call<F>(&mut self, func: &F) -> Result<F::Result, Error>
    where
        F: Call + Serialize,
        F::Result: Deserialize,
    {
        let mut buf = Buffer::new();
        self.protocol.pack(&mut buf, func);
        self.transport.pack(&mut buf);
        self.stream.write_all(&buf).await?;

        let mut buf = Buffer::new();
        self.recv_buf(&mut buf).await?;
        let result = self.protocol.unpack(&buf)?;

        Ok(result)
    }

    async fn recv_buf(&mut self, buf: &mut Buffer) -> Result<(), Error> {
        loop {
            match self.transport.unpack(buf) {
                Ok(()) => break Ok(()),
                Err(durov_mtproto::transports::Error::MissingBytes(missing)) => {
                    let pos = buf.len();
                    buf.resize_back(missing);
                    self.stream.read_exact(&mut buf[pos..]).await?;
                }
                Err(err) => break Err(err.into()),
            }
        }
    }
}

impl<T: Transport> PlainClient<T>
where
    T: Send + 'static,
{
    pub async fn auth(mut self) -> Result<(EncryptedClient, [u8; 256]), Error> {
        let mut attempt = 0;
        let (step1, step2, step3) = loop {
            attempt += 1;

            let step1 = auth::step1();
            let res = self.call(&step1.req).await?;

            let step2 = auth::step2(res, step1.nonce, &self.config.dc)?;
            let res = self.call(&step2.req).await?;

            match auth::step3(res, step1.nonce, step2.server_nonce, step2.new_nonce) {
                Ok(step3) => {
                    self.protocol.set_server_time(step3.server_time as f64);
                    break (step1, step2, step3);
                }
                Err(auth::Error::Restart) => {
                    if attempt >= 3 {
                        return Err(Error::AuthFailed);
                    }
                    log::warn!("restarting auth: attempt {attempt}");
                }
                Err(err) => return Err(err.into()),
            }
        };

        let mut attempt = 0;
        let mut prev_auth_key_aux_id = None;
        loop {
            attempt += 1;

            let step4 = auth::step4(
                step1.nonce,
                step2.server_nonce,
                step3.tmp_aes_key,
                step3.tmp_aes_iv,
                &step3.p,
                &step3.g,
                &step3.g_a,
                prev_auth_key_aux_id,
            )?;
            let res = self.call(&step4.req).await?;

            match auth::step5(
                res,
                step1.nonce,
                step2.server_nonce,
                step2.new_nonce,
                &step4.auth_key,
            ) {
                Ok(step5) => {
                    let encrypted = self.upgrade(step4.auth_key, step5.server_salt);
                    break Ok((encrypted, step4.auth_key));
                }
                Err(auth::Error::RetryStep4 { auth_key_aux_id }) => {
                    if attempt >= 3 {
                        return Err(Error::AuthFailed);
                    }
                    prev_auth_key_aux_id = Some(auth_key_aux_id);
                    log::warn!("restarting auth step 4: attempt {attempt}");
                }
                Err(err) => return Err(err.into()),
            }
        }
    }

    fn upgrade(self, auth_key: [u8; 256], salt: i64) -> EncryptedClient {
        let protocol = Encrypted::from_plain(
            self.protocol,
            auth_key,
            salt,
            self.config.use_gzip,
        );
        EncryptedClient::new(self.stream, self.transport, protocol)
    }
}
