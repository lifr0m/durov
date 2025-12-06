use crate::encrypted::EncryptedClient;
use crate::{tcp, Error, MtConfig};
use durov_mtproto::auth;
use durov_mtproto::protocols::encrypted::Encrypted;
use durov_mtproto::protocols::plain::Plain;
use durov_mtproto::transports::Transport;
use durov_tl_types::buffer::Buffer;
use durov_tl_types::serialize::Serialize;
use durov_tl_types::Call;
use tokio::io;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

pub struct PlainClient<T> {
    config: MtConfig,
    stream: TcpStream,
    transport: T,
    protocol: Plain,
}

impl<T: Transport> PlainClient<T> {
    pub async fn connect(config: MtConfig) -> io::Result<Self> {
        let stream = tcp::connect(config.dc.host, config.dc.port).await?;
        let transport = T::default();
        let protocol = Plain::new();
        Ok(Self { config, stream, transport, protocol })
    }

    pub async fn call<F>(&mut self, func: &F) -> Result<F::Result, Error>
    where
        F: Call + Serialize,
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
        let step1 = auth::step1();
        let res = self.call(&step1.req).await?;

        let step2 = auth::step2(res, step1.nonce, self.config.dc)?;
        let res = self.call(&step2.req).await?;

        let step3 = auth::step3(res, step1.nonce, step2.server_nonce, step2.new_nonce)?;
        self.protocol.set_server_time(step3.server_time as f64);

        let step4 = auth::step4(
            step1.nonce,
            step2.server_nonce,
            step3.tmp_aes_key,
            step3.tmp_aes_iv,
            &step3.p,
            &step3.g,
            &step3.g_a,
            None,
        )?;
        let res = self.call(&step4.req).await?;

        let step5 = auth::step5(
            res,
            step1.nonce,
            step2.server_nonce,
            step2.new_nonce,
            &step4.auth_key,
        )?;
        Ok((self.upgrade(step4.auth_key, step5.server_salt), step4.auth_key))
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
