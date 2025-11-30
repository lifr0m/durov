use rsa::pkcs1::DecodeRsaPublicKey;

pub enum DatacenterType {
    Production,
    Test,
    Media,
}

pub struct Datacenter {
    pub id: i32,
    pub typ: DatacenterType,
    pub host: String,
    pub port: u16,
    pub pubkey: rsa::RsaPublicKey,
}

pub fn parse_pubkey(pubkey: &str) -> rsa::pkcs1::Result<rsa::RsaPublicKey> {
    rsa::RsaPublicKey::from_pkcs1_pem(pubkey)
}
