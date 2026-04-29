use crate::proxy::{Proxy, Socks5Auth};
use durov_mtproto::datacenter::Datacenter;
use tokio::io;
use tokio::net::TcpStream;

pub async fn connect(dc: &Datacenter, proxy: Option<&Proxy>) -> io::Result<TcpStream> {
    match proxy {
        None => connect_plain(dc).await,
        Some(Proxy::Socks5 {
            host,
            port,
            auth,
        }) => connect_socks5(dc, host, *port, auth.as_ref()).await,
    }
}

async fn connect_plain(dc: &Datacenter) -> io::Result<TcpStream> {
    TcpStream::connect((dc.host.as_ref(), dc.port)).await
}

async fn connect_socks5(dc: &Datacenter, host: &str, port: u16, auth: Option<&Socks5Auth>)
    -> io::Result<TcpStream>
{
    let result = match auth {
        Some(auth) => fast_socks5::client::Socks5Stream::connect_with_password(
            (host, port),
            dc.host.clone(),
            dc.port,
            auth.username.clone(),
            auth.password.clone(),
            fast_socks5::client::Config::default(),
        ).await,
        None => fast_socks5::client::Socks5Stream::connect(
            (host, port),
            dc.host.clone(),
            dc.port,
            fast_socks5::client::Config::default(),
        ).await,
    };
    match result {
        Ok(stream) => Ok(stream.get_socket()),
        Err(err) => Err(io::Error::other(format!("socks5: {err}"))),
    }
}
