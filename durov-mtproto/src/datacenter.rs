#[derive(Debug, Clone)]
pub struct Datacenter {
    pub id: i32,
    pub prod: bool,
    pub host: String,
    pub port: u16,
    pub pubkey: &'static str,
}
