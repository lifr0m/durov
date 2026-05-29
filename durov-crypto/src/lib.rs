pub mod auth;
pub mod srp;
pub mod encrypted;
pub mod primitives;

#[derive(Clone)]
pub struct Datacenter {
    pub id: i32,
    pub host: String,
    pub port: u16,
    pub pubkey: &'static str,
}
