use tokio::io;
use tokio::net::TcpStream;

pub async fn connect(host: &str, port: u16) -> io::Result<TcpStream> {
    TcpStream::connect((host, port)).await
}
