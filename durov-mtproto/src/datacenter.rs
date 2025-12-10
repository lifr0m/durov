#[derive(Copy, Clone)]
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
    pub pubkey: &'static str,
}
