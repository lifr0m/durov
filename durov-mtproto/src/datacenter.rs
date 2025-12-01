pub enum DatacenterType {
    Production,
    Test,
    Media,
}

pub struct Datacenter {
    pub id: i32,
    pub typ: DatacenterType,
    pub host: &'static str,
    pub port: u16,
}
