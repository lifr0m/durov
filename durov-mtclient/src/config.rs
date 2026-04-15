use durov_mtproto::datacenter::Datacenter;

pub struct MtConfig {
    pub dc: Datacenter,
    pub use_gzip: bool,
    pub updates: bool,
}
