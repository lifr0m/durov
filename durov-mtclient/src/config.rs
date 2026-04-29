use crate::proxy::Proxy;
use durov_mtproto::datacenter::Datacenter;

pub struct MtConfig {
    pub dc: Datacenter,
    pub proxy: Option<Proxy>,
    pub use_gzip: bool,
    pub updates: bool,
}
