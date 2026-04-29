#[derive(Clone)]
pub struct Socks5Auth {
    pub username: String,
    pub password: String,
}

#[derive(Clone)]
pub enum Proxy {
    Socks5 {
        host: String,
        port: u16,
        auth: Option<Socks5Auth>,
    },
}
